use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::cell::RefCell;

use wie_backend::RecordId;

use crate::indexed_db_store::{Store, StoreKey};

/// One app's IndexedDB database, read once into memory. Record store calls are synchronous on a phone and games call
/// them in loops (a record per frame would take seconds), so reads answer from memory; writes also go to IndexedDB.
struct AppDatabase {
    store: Store,
    /// Record store names (key [name, ""]).
    names: BTreeSet<String>,
    /// Records (key "<name><id>").
    records: BTreeMap<String, Vec<u8>>,
}

type SharedAppDatabase = Rc<RefCell<AppDatabase>>;

pub struct DatabaseRepository {
    apps: RefCell<BTreeMap<String, SharedAppDatabase>>,
}

// single threaded wasm; RefCell + Rc are only touched sequentially and never borrowed across an await.
unsafe impl Send for DatabaseRepository {}
unsafe impl Sync for DatabaseRepository {}

impl DatabaseRepository {
    pub fn new() -> Self {
        Self {
            apps: RefCell::new(BTreeMap::new()),
        }
    }

    async fn app(&self, app_id: &str) -> SharedAppDatabase {
        let db_name = format!("wie_{app_id}");
        if let Some(app) = self.apps.borrow().get(&db_name) {
            return app.clone();
        }

        let store = Store::open(&db_name, &db_name).await;
        let mut names = BTreeSet::new();
        let mut records = BTreeMap::new();
        for (key, value) in store.get_all_entries().await {
            match key {
                StoreKey::String(key) => {
                    records.insert(key, value);
                }
                StoreKey::Pair(name, suffix) if suffix.is_empty() => {
                    names.insert(name);
                }
                StoreKey::Pair(_, _) => {}
            }
        }
        let app = Rc::new(RefCell::new(AppDatabase { store, names, records }));

        self.apps.borrow_mut().entry(db_name).or_insert(app).clone()
    }
}

#[async_trait::async_trait]
impl wie_backend::DatabaseRepository for DatabaseRepository {
    async fn create(&self, name: &str, app_id: &str) {
        let store = {
            let app = self.app(app_id).await;
            if !app.borrow_mut().names.insert(name.to_string()) {
                return;
            }
            app.borrow().store.clone()
        };
        store.set(StoreKey::Pair(name.to_string(), String::new()), &[]).await;
    }

    async fn open(&self, name: &str, app_id: &str) -> Box<dyn wie_backend::Database> {
        Box::new(Database {
            app: self.app(app_id).await,
            key_prefix: name.to_string(),
        })
    }

    async fn exists(&self, name: &str, app_id: &str) -> bool {
        let app = self.app(app_id).await;
        let app = app.borrow();
        app.names.contains(name)
            || app
                .records
                .keys()
                .any(|key| key.strip_prefix(name).is_some_and(|id| id.parse::<RecordId>().is_ok()))
    }

    async fn delete(&self, _name: &str, _app_id: &str) -> bool {
        true // TODO
    }
}

pub struct Database {
    app: SharedAppDatabase,
    key_prefix: String,
}

// single threaded wasm, see DatabaseRepository.
unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
    fn record_key(&self, id: RecordId) -> String {
        format!("{}{}", self.key_prefix, id)
    }
}

#[async_trait::async_trait]
impl wie_backend::Database for Database {
    async fn add(&mut self, data: &[u8]) -> RecordId {
        let id = self.next_id().await;
        self.set(id, data).await;

        id
    }

    async fn next_id(&self) -> RecordId {
        let ids = self.get_record_ids().await;

        ids.iter().max().map_or(1, |&id| id + 1)
    }

    async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
        self.app.borrow().records.get(&self.record_key(id)).cloned()
    }

    async fn set(&mut self, id: RecordId, data: &[u8]) -> bool {
        let key = self.record_key(id);
        self.app.borrow_mut().records.insert(key.clone(), data.to_vec());
        let store = self.app.borrow().store.clone();
        store.set(StoreKey::String(key), data).await;
        true
    }

    async fn delete(&mut self, id: RecordId) -> bool {
        let key = self.record_key(id);
        self.app.borrow_mut().records.remove(&key);
        let store = self.app.borrow().store.clone();
        store.delete(StoreKey::String(key)).await;
        true
    }

    async fn get_record_ids(&self) -> Vec<RecordId> {
        self.app
            .borrow()
            .records
            .keys()
            .filter_map(|key| key.strip_prefix(self.key_prefix.as_str()).and_then(|tail| tail.parse::<RecordId>().ok()))
            .collect()
    }
}
