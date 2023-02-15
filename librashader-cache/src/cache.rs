use crate::cacheable::Cacheable;
use crate::key::CacheKey;
use platform_dirs::AppDirs;
use rusqlite::{params, Connection, DatabaseName};
use std::error::Error;
use std::path::PathBuf;

pub fn get_cache_dir() -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir =
        if let Some(cache_dir) = AppDirs::new(Some("librashader"), false).map(|a| a.cache_dir) {
            cache_dir
        } else {
            let mut current_dir = std::env::current_dir()?;
            current_dir.push("librashader");
            current_dir
        };

    std::fs::create_dir_all(&cache_dir)?;

    Ok(cache_dir)
}

pub fn get_cache() -> Result<Connection, Box<dyn Error>> {
    let cache_dir = get_cache_dir()?;
    let mut conn = Connection::open(&cache_dir.join("librashader.db"))?;

    let tx = conn.transaction()?;
    tx.pragma_update(Some(DatabaseName::Main), "journal_mode", "wal2")?;
    tx.execute(
        r#"create table if not exists cache (
        type text not null,
        id blob not null,
        value blob not null unique,
        primary key (id, type)
    )"#,
        [],
    )?;
    tx.commit()?;
    Ok(conn)
}

pub(crate) fn get_blob(
    conn: &Connection,
    index: &str,
    key: &[u8],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let value = conn.query_row(
        &*format!("select value from cache where (type = (?1) and id = (?2))"),
        params![index, key],
        |row| row.get(0),
    )?;
    Ok(value)
}

pub(crate) fn set_blob(conn: &Connection, index: &str, key: &[u8], value: &[u8]) {
    match conn.execute(
        &*format!("insert or replace into cache (type, id, value) values (?1, ?2, ?3)"),
        params![index, key, value],
    ) {
        Ok(_) => return,
        Err(e) => println!("err: {:?}", e),
    }
}

pub fn get_cached_blob<T, H, const KEY_SIZE: usize>(
    index: &str,
    key: &[H; KEY_SIZE],
    transform: impl FnOnce(Vec<u8>) -> T,
) -> Option<T>
where
    H: CacheKey,
{
    let cache = get_cache();

    let Ok(cache) = cache else {
        return None
    };

    let key = {
        let mut hasher = blake3::Hasher::new();
        for subkeys in key {
            hasher.update(subkeys.hash_bytes());
        }
        let hash = hasher.finalize();
        hash
    };

    let Ok(blob) = get_blob(&cache, index, key.as_bytes()) else {
        return None;
    };

    Some(transform(blob))
}

pub fn cache_object<E, T, R, H, const KEY_SIZE: usize>(
    index: &str,
    keys: &[H; KEY_SIZE],
    factory: impl FnOnce(&[H; KEY_SIZE]) -> Result<T, E>,
    attempt: impl Fn(T) -> Result<R, E>,
    do_cache: bool,
) -> Result<R, E>
where
    H: CacheKey,
    T: Cacheable,
{
    if !do_cache {
        return Ok(attempt(factory(keys)?)?);
    }

    let cache = get_cache();

    let Ok(cache) = cache else {
        return Ok(attempt(factory(keys)?)?);
    };

    let hashkey = {
        let mut hasher = blake3::Hasher::new();
        for subkeys in keys {
            hasher.update(subkeys.hash_bytes());
        }
        let hash = hasher.finalize();
        hash
    };

    'attempt: {
        if let Ok(blob) = get_blob(&cache, index, hashkey.as_bytes()) {
            let cached = T::from_bytes(&blob).map(&attempt);

            match cached {
                None => break 'attempt,
                Some(Err(_)) => break 'attempt,
                Some(Ok(res)) => return Ok(res),
            }
        }
    };

    let blob = factory(keys)?;

    if let Some(slice) = T::to_bytes(&blob) {
        set_blob(&cache, index, hashkey.as_bytes(), &slice);
    }
    Ok(attempt(blob)?)
}

pub fn cache_pipeline<E, T, R, const KEY_SIZE: usize>(
    index: &str,
    keys: &[&dyn CacheKey; KEY_SIZE],
    attempt: impl Fn(Option<Vec<u8>>) -> Result<R, E>,
    factory: impl FnOnce(&R) -> Result<T, E>,
    do_cache: bool,
) -> Result<R, E>
where
    T: Cacheable,
{
    if !do_cache {
        return Ok(attempt(None)?);
    }

    let cache = get_cache();

    let Ok(cache) = cache else {
        return Ok(attempt(None)?);
    };

    let hashkey = {
        let mut hasher = blake3::Hasher::new();
        for subkeys in keys {
            hasher.update(subkeys.hash_bytes());
        }
        let hash = hasher.finalize();
        hash
    };

    let pipeline = 'attempt: {
        if let Ok(blob) = get_blob(&cache, index, hashkey.as_bytes()) {
            let cached = attempt(Some(blob));
            match cached {
                Ok(res) => {
                    break 'attempt res;
                }
                _ => (),
            }
        }

        attempt(None)?
    };

    // update the pso every time just in case.
    if let Ok(state) = factory(&pipeline) {
        if let Some(slice) = T::to_bytes(&state) {
            set_blob(&cache, index, hashkey.as_bytes(), &slice);
        }
    }

    Ok(pipeline)
}
