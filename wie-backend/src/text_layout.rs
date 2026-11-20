use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::canvas::{Font, string_width};

pub fn minimum_width(font: &Font, text: &str, pt_size: f32) -> i32 {
    text.chars()
        .filter(|character| *character != '\n')
        .map(|character| string_width(font, &character.to_string(), pt_size).ceil() as i32)
        .max()
        .unwrap_or(0)
}

pub fn preferred_width(font: &Font, text: &str, pt_size: f32) -> i32 {
    text.split('\n')
        .map(|line| string_width(font, line, pt_size).ceil() as i32)
        .max()
        .unwrap_or(0)
}

pub fn wrap(font: &Font, text: &str, pt_size: f32, maximum_width: Option<i32>) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let characters = paragraph.chars().collect::<Vec<_>>();
        if characters.is_empty() {
            lines.push(String::new());
            continue;
        }

        let Some(maximum_width) = maximum_width else {
            lines.push(paragraph.to_string());
            continue;
        };
        let maximum_width = maximum_width.max(1);
        let mut start = 0;
        while start < characters.len() {
            let mut end = start;
            let mut line = String::new();
            let mut word_boundary = None;
            while end < characters.len() {
                // The width of the whole line, as preferred_width measures it: rounding each character up would
                // make a line wider than the width its item was given for it.
                line.push(characters[end]);
                if end > start && string_width(font, &line, pt_size).ceil() as i32 > maximum_width {
                    break;
                }
                end += 1;
                if characters[end - 1].is_whitespace() {
                    word_boundary = Some(end);
                }
            }

            let split = if end == characters.len() {
                end
            } else {
                word_boundary.filter(|boundary| *boundary > start).unwrap_or(end.max(start + 1))
            };
            let line = characters[start..split].iter().collect::<String>();
            lines.push(line.trim_end().to_string());
            start = split;
            while start < characters.len() && characters[start].is_whitespace() {
                start += 1;
            }
        }
    }

    lines
}

#[cfg(test)]
mod test {
    use crate::canvas::{Font, string_width};

    use super::{preferred_width, wrap};

    #[test]
    fn preserves_explicit_newlines_and_prefers_word_boundaries() {
        let font = Font::try_from_static(include_bytes!("../../assets/neodgm.ttf")).unwrap();
        let width = string_width(&font, "alpha ", 10.0).ceil() as i32;
        assert_eq!(wrap(&font, "alpha beta\n\ngamma", 10.0, Some(width)), ["alpha", "beta", "", "gamma"]);
    }

    #[test]
    fn falls_back_to_character_boundaries_for_long_words() {
        let font = Font::try_from_static(include_bytes!("../../assets/neodgm.ttf")).unwrap();
        let width = string_width(&font, "ab", 10.0).ceil() as i32;
        assert_eq!(wrap(&font, "abcdef", 10.0, Some(width)), ["ab", "cd", "ef"]);
    }

    #[test]
    fn a_line_fits_its_preferred_width() {
        let font = Font::try_from_static(include_bytes!("../../assets/neodgm.ttf")).unwrap();
        for text in ["Outdoor", "Julian", "Moorhen", "Doug", "iiiiiiil"] {
            let width = preferred_width(&font, text, 10.0);
            assert_eq!(wrap(&font, text, 10.0, Some(width)), [text]);
        }
    }
}
