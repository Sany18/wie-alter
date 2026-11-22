use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::btree_map::BTreeMap,
    format, str,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use jvm::{Result as JvmResult, runtime::JavaLangString};

use wie_backend::{DefaultTaskRunner, Emulator, Event, Platform, System, extract_zip};
use wie_jvm_support::{JvmSupport, RustJavaJvmImplementation};
use wie_util::{Result, WieError};

// Vendor classes a jar may replace with its own (see load()).
const REPLACEABLE: [&str; 1] = ["com/nokia/mid/ui/FullCanvas"];

// The super class of a class file: walks the constant pool to access_flags, this_class, super_class.
fn super_class_name(class: &[u8]) -> Option<String> {
    let u16_at = |at: usize| Some(u16::from_be_bytes([*class.get(at)?, *class.get(at + 1)?]));
    if class.get(0..4)? != [0xca, 0xfe, 0xba, 0xbe] {
        return None;
    }
    let count = u16_at(8)? as usize;
    let mut entries: Vec<(u8, usize)> = vec![(0, 0); count];
    let mut at = 10;
    let mut index = 1;
    while index < count {
        let tag = *class.get(at)?;
        entries[index] = (tag, at);
        at += 1 + match tag {
            1 => 2 + u16_at(at + 1)? as usize,
            7 | 8 | 16 | 19 | 20 => 2,
            15 => 3,
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => 4,
            5 | 6 => 8,
            _ => return None,
        };
        index += if tag == 5 || tag == 6 { 2 } else { 1 };
    }
    let super_index = u16_at(at + 4)? as usize;
    let (tag, offset) = *entries.get(super_index)?;
    if tag != 7 {
        return None;
    }
    let (tag, offset) = *entries.get(u16_at(offset + 1)? as usize)?;
    if tag != 1 {
        return None;
    }
    let length = u16_at(offset + 1)? as usize;
    str::from_utf8(class.get(offset + 3..offset + 3 + length)?).ok().map(ToString::to_string)
}

pub struct J2MEEmulator {
    system: System,
}

impl J2MEEmulator {
    pub fn jar_metadata(jar: &[u8]) -> Result<Option<(String, Option<Vec<u8>>)>> {
        let files = extract_zip(jar)?;
        let Some((_, manifest)) = files.iter().find(|(path, _)| path.eq_ignore_ascii_case("META-INF/MANIFEST.MF")) else {
            return Ok(None);
        };
        let descriptor = J2MEDescriptor::parse(manifest);
        if descriptor.name.is_empty() || descriptor.main_class_name.is_empty() {
            return Ok(None);
        }

        let icon = if descriptor.icon.is_empty() {
            None
        } else {
            files.get(descriptor.icon.trim_start_matches('/')).cloned()
        };

        Ok(Some((descriptor.name, icon)))
    }

    pub fn from_jad_jar(platform: Box<dyn Platform>, jad: Vec<u8>, jar_filename: String, jar: Vec<u8>) -> Result<Self> {
        Self::from_jar_with_descriptor(platform, &jar_filename, jar, Some(&jad), BTreeMap::new())
    }

    pub fn from_jar(platform: Box<dyn Platform>, jar_filename: &str, jar: Vec<u8>) -> Result<Self> {
        Self::from_jar_with_descriptor(platform, jar_filename, jar, None, BTreeMap::new())
    }

    // The application descriptor (JAD), when there is one, lies over the jar's manifest: an attribute in both takes the
    // descriptor's value, as in MIDP 1.0. The system properties the host gives override the platform's defaults.
    pub fn from_jar_with_descriptor(
        platform: Box<dyn Platform>,
        jar_filename: &str,
        jar: Vec<u8>,
        jad: Option<&[u8]>,
        system_properties: BTreeMap<String, String>,
    ) -> Result<Self> {
        let contents = extract_zip(&jar)?;
        let manifest = contents
            .iter()
            .find(|(path, _)| path.eq_ignore_ascii_case("META-INF/MANIFEST.MF"))
            .map(|(_, manifest)| J2MEDescriptor::parse(manifest).properties);
        let mut properties = match (manifest, jad) {
            (Some(manifest), _) => manifest,
            (None, Some(_)) => BTreeMap::new(),
            (None, None) => return Err(WieError::FatalError("JAR manifest not found".into())),
        };
        if let Some(jad) = jad {
            properties.extend(J2MEDescriptor::parse(jad).properties);
        }
        let descriptor = J2MEDescriptor::from_properties(properties);
        let files = [(jar_filename.to_owned(), jar)].into_iter().collect();

        Self::load(platform, jar_filename, descriptor, &files, system_properties)
    }

    fn load(
        platform: Box<dyn Platform>,
        jar_filename: &str,
        descriptor: J2MEDescriptor,
        files: &BTreeMap<String, Vec<u8>>,
        system_properties: BTreeMap<String, String>,
    ) -> Result<Self> {
        if descriptor.name.is_empty() || descriptor.main_class_name.is_empty() {
            return Err(WieError::FatalError("MIDlet-1 name or main class not found".into()));
        }
        let system = System::new(platform, &descriptor.name, &descriptor.name, DefaultTaskRunner);

        for (path, data) in files {
            system.filesystem().add_virtual(path, data.clone());
        }

        // A game converted for another phone may ship its own com.nokia.mid.ui.FullCanvas built on that phone's API
        // (a Motorola canvas whose keyPressed feeds the game's KEYPRESSED). On such a phone the jar's class is the one
        // that runs; ours would cut the game off from its keys. A FullCanvas straight on Canvas is only a full-screen
        // shim: ours does the same and more, so it stays.
        let replaced = files
            .get(jar_filename)
            .and_then(|jar| extract_zip(jar).ok())
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|(path, data)| {
                        let name = path.strip_suffix(".class")?;
                        let parent = super_class_name(data)?;
                        (REPLACEABLE.contains(&name) && parent != "javax/microedition/lcdui/Canvas").then(|| name.to_string())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut system_clone = system.clone();
        let jar_filename = jar_filename.to_owned();

        system.spawn(async move || {
            Self::do_start(
                &mut system_clone,
                jar_filename,
                descriptor.properties,
                system_properties,
                descriptor.main_class_name,
                replaced,
            )
            .await
        });

        Ok(J2MEEmulator { system })
    }

    #[tracing::instrument(name = "start", skip_all)]
    async fn do_start(
        system: &mut System,
        jar_filename: String,
        properties: BTreeMap<String, String>,
        system_properties: BTreeMap<String, String>,
        main_class_name: String,
        replaced: Vec<String>,
    ) -> Result<()> {
        let properties = properties
            .into_iter()
            .map(|(k, v)| (format!("wie.appProperty.{k}"), v))
            .collect::<Vec<_>>();
        // The key codes of most MIDP phones outside Korea (Nokia's, -1 to -7 for the arrows, select and soft keys), and
        // CLDC's default encoding: one char per byte, as the games outside Korea expect from new String(bytes) and getBytes().
        let properties = [
            ("microedition.platform", "wie"),
            ("wie.keyCodes", "nokia"),
            ("file.encoding", "ISO-8859-1"),
        ]
        .into_iter()
        .chain(system_properties.iter().map(|(k, v)| (k.as_ref(), v.as_ref())))
        .chain(properties.iter().map(|(k, v)| (k.as_ref(), v.as_ref())))
        .collect::<Vec<_>>();

        let vendor = wie_midp_vendor::get_protos()
            .into_iter()
            .filter(|proto| {
                let shipped = replaced.iter().any(|name| name == proto.name);
                if shipped {
                    tracing::info!("{} comes from the jar", proto.name);
                }
                !shipped
            })
            .collect::<Vec<_>>();
        let protos = [wie_midp::get_protos().into(), vendor.into_boxed_slice()];
        let jvm = JvmSupport::new_jvm(system, Some(&jar_filename), Box::new(protos), &properties, RustJavaJvmImplementation).await?;

        let main_class_name = main_class_name.replace('.', "/");

        // Resolve with the system loader before entering the rustjar-loaded Launcher.
        if let Err(error) = jvm.resolve_class(&main_class_name).await {
            return Err(JvmSupport::to_wie_err(&jvm, error).await);
        }
        let main_class_java = JavaLangString::from_rust_string(&jvm, &main_class_name).await.unwrap();

        let result: JvmResult<()> = jvm
            .invoke_static("net/wie/Launcher", "start", "(Ljava/lang/String;)V", (main_class_java,))
            .await;

        if let Err(x) = result {
            return Err(JvmSupport::to_wie_err(&jvm, x).await);
        }

        Ok(())
    }
}

impl Emulator for J2MEEmulator {
    fn is_preparing(&self) -> bool {
        false
    }

    fn handle_event(&mut self, event: Event) {
        self.system.event_queue().push(event)
    }

    fn tick(&mut self) -> Result<()> {
        self.system.tick()
    }
}

struct J2MEDescriptor {
    name: String,
    main_class_name: String,
    icon: String,
    properties: BTreeMap<String, String>,
}

impl J2MEDescriptor {
    pub fn parse(data: &[u8]) -> Self {
        let lines = data.split(|x| *x == b'\n');

        let mut properties = BTreeMap::new();
        let mut logical_lines: Vec<String> = Vec::new();

        for line in lines {
            let Ok(line) = str::from_utf8(line) else {
                continue;
            };
            let line = line.trim_end_matches('\r');

            if let Some(continuation) = line.strip_prefix(' ') {
                if let Some(previous) = logical_lines.last_mut() {
                    previous.push_str(continuation);
                }
            } else {
                logical_lines.push(line.to_string());
            }
        }

        for line in logical_lines {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            properties.insert(key.to_string(), value.to_string());
        }

        Self::from_properties(properties)
    }

    fn from_properties(properties: BTreeMap<String, String>) -> Self {
        let mut name = String::new();
        let mut main_class_name = String::new();
        let mut icon = String::new();
        let mut midlet_icon = String::new();

        for (key, value) in &properties {
            match key.as_str() {
                "MIDlet-Icon" => icon = value.to_string(),
                "MIDlet-1" => {
                    let mut values = value.split(',');
                    if let Some(value) = values.next() {
                        name = value.trim().to_string();
                    }
                    if let Some(value) = values.next() {
                        midlet_icon = value.trim().to_string();
                    }
                    if let Some(value) = values.next() {
                        main_class_name = value.trim().to_string();
                    }
                }
                _ => {}
            }
        }

        if icon.is_empty() {
            icon = midlet_icon;
        }

        Self {
            name,
            main_class_name,
            icon,
            properties,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
    use std::io::{Cursor, Write};

    use test_utils::TestPlatform;
    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::{J2MEDescriptor, J2MEEmulator};

    #[test]
    fn jar_and_jad_use_midlet_name_for_storage() {
        let manifest = b"MIDlet-Name: My MIDlet Suite\nMIDlet-1: App title, , example.Main\n";
        for path in ["META-INF/MANIFEST.MF", "meta-inf/manifest.mf", "Meta-Inf/Manifest.Mf"] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer.start_file(path, SimpleFileOptions::default()).unwrap();
            writer.write_all(manifest).unwrap();
            let jar = writer.finish().unwrap().into_inner();
            let (title, _) = J2MEEmulator::jar_metadata(&jar).unwrap().unwrap();
            assert_eq!(title, "App title");

            for filename in ["original.jar", "renamed.jar"] {
                let emulator = J2MEEmulator::from_jar(Box::new(TestPlatform::new()), filename, jar.clone()).unwrap();
                assert_eq!(emulator.system.pid(), "App title");
                assert_eq!(emulator.system.aid(), "App title");
                let emulator =
                    J2MEEmulator::from_jad_jar(Box::new(TestPlatform::new()), manifest.to_vec(), filename.to_string(), jar.clone()).unwrap();
                assert_eq!(emulator.system.pid(), "App title");
                assert_eq!(emulator.system.aid(), "App title");
            }
        }
    }

    #[test]
    fn jad_attributes_lie_over_the_manifest() {
        let jar = |manifest: Option<&[u8]>| {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            if let Some(manifest) = manifest {
                writer.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default()).unwrap();
                writer.write_all(manifest).unwrap();
            }
            writer.start_file("example/Main.class", SimpleFileOptions::default()).unwrap();
            writer.finish().unwrap().into_inner()
        };
        let with_manifest = jar(Some(b"MIDlet-Name: Suite\nMIDlet-1: Manifest title, , example.Main\n"));
        let jad: &[u8] = b"MIDlet-1: Descriptor title, , example.Main\nGame-Levels: 9\n";

        let emulator = J2MEEmulator::from_jar_with_descriptor(
            Box::new(TestPlatform::new()),
            "app.jar",
            with_manifest.clone(),
            Some(jad),
            BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(emulator.system.pid(), "Descriptor title");
        let emulator =
            J2MEEmulator::from_jar_with_descriptor(Box::new(TestPlatform::new()), "app.jar", with_manifest, None, BTreeMap::new()).unwrap();
        assert_eq!(emulator.system.pid(), "Manifest title");
        let emulator =
            J2MEEmulator::from_jar_with_descriptor(Box::new(TestPlatform::new()), "app.jar", jar(None), Some(jad), BTreeMap::new()).unwrap();
        assert_eq!(emulator.system.pid(), "Descriptor title");
        assert!(J2MEEmulator::from_jar_with_descriptor(Box::new(TestPlatform::new()), "app.jar", jar(None), None, BTreeMap::new()).is_err());
    }

    #[test]
    fn parses_utf8_midlet_metadata() {
        let descriptor = J2MEDescriptor::parse(
            "Manifest-Version: 1.0\r\nMIDlet-Name: 모바일 앱\r\nMIDlet-Icon: /app.png\r\nMIDlet-1: 모바일 앱, /midlet.png, example.Main\r\n"
                .as_bytes(),
        );

        assert_eq!(descriptor.name, "모바일 앱");
        assert_eq!(descriptor.main_class_name, "example.Main");
        assert_eq!(descriptor.icon, "/app.png");
    }

    #[test]
    fn ignores_malformed_manifest_lines() {
        let descriptor = J2MEDescriptor::parse(b"invalid line\nMIDlet-1: incomplete\n\xff\nMIDlet-Name: Valid App\n");

        assert_eq!(descriptor.name, "incomplete");
        assert!(descriptor.main_class_name.is_empty());
    }

    #[test]
    fn parses_midlet_icon_and_continued_manifest_value() {
        let descriptor =
            J2MEDescriptor::parse(b"MIDlet-Name: Boulder Dash\nMIDlet-1: Boulder Dash, icon.png, net.instantcom.boulderdash.BoulderDa\n shMIDlet\n");

        assert_eq!(descriptor.main_class_name, "net.instantcom.boulderdash.BoulderDashMIDlet");
        assert_eq!(descriptor.icon, "icon.png");
    }
}

#[cfg(test)]
mod super_class_tests {
    use alloc::vec::Vec;

    use super::super_class_name;

    // A class file header and constant pool: 1 Utf8 this, 2 Class #1, 3 Long (two slots), 5 Utf8 parent, 6 Class #5.
    fn class(parent: &str) -> Vec<u8> {
        let mut data = Vec::from([0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 49, 0, 7]);
        let utf8 = |data: &mut Vec<u8>, text: &str| {
            data.push(1);
            data.extend((text.len() as u16).to_be_bytes());
            data.extend(text.as_bytes());
        };
        utf8(&mut data, "com/nokia/mid/ui/FullCanvas");
        data.extend([7, 0, 1]);
        data.extend([5, 0, 0, 0, 0, 0, 0, 0, 1]);
        utf8(&mut data, parent);
        data.extend([7, 0, 5]);
        data.extend([0, 0x21, 0, 2, 0, 6]); // access flags, this_class, super_class
        data
    }

    #[test]
    fn reads_the_super_class() {
        assert_eq!(
            super_class_name(&class("com/hellomoto/fullscreen/FullCn")).as_deref(),
            Some("com/hellomoto/fullscreen/FullCn")
        );
        assert_eq!(
            super_class_name(&class("javax/microedition/lcdui/Canvas")).as_deref(),
            Some("javax/microedition/lcdui/Canvas")
        );
        assert_eq!(super_class_name(b"not a class"), None);
    }
}
