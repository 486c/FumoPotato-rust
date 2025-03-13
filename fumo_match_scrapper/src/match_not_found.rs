use std::{collections::HashSet, fs::{File, OpenOptions}, io::{BufRead, BufReader, BufWriter, Write}, path::PathBuf, time::SystemTime};

use tokio::sync::RwLock;

pub const MATCH_NOT_FOUND_FILE_PATH: &str = "./fumo_match_not_found";

pub struct MatchNotFoundList {
    // Might leave a large memory footprint when having a lot of integers.
    // Still better than keeping then in DB :)
    //
    // Using u32 to reduce used memory even more
    list: RwLock<HashSet<u32>>, 

    modified: SystemTime,
}

impl MatchNotFoundList {
    pub fn new() -> eyre::Result<Self> {
        let (list, modified_time) = if !PathBuf::from(MATCH_NOT_FOUND_FILE_PATH).exists() {
            (HashSet::new(), SystemTime::now())
        } else {
            let file = File::open(MATCH_NOT_FOUND_FILE_PATH)?;

            let stat = file.metadata()?;
            let mut reader = BufReader::new(file);

            let inner = Self::reader_to_list(&mut reader);

            (inner, stat.modified()?)
        };


        Ok(Self {
            list: list.into(),
            modified: modified_time,
        })
    }
    
    pub async fn check(&self, match_id: i64) -> bool {
        let lock = self.list.read().await;

        lock.contains(&(match_id as u32))
    }

    pub async fn insert(&self, match_id: i64) {
        let mut lock = self.list.write().await;

        lock.insert(match_id as u32);
    }

    fn reader_to_list(reader: &mut BufReader<File>) -> HashSet<u32> {
        let mut inner = HashSet::new();

        reader.split(b',')
            .flatten()
            .map(|split| {
                let split_str = unsafe { std::str::from_utf8_unchecked(&split) };
                split_str.parse::<u32>()
            })
            .flatten()
            .for_each(|num| {
                inner.insert(num);
            });

        inner
    }

    pub async fn close(&self) -> eyre::Result<()> {
        let is_exists = PathBuf::from(MATCH_NOT_FOUND_FILE_PATH).exists();
        let file = File::open(MATCH_NOT_FOUND_FILE_PATH)?;
        let stat = file.metadata()?;

        if self.modified < stat.modified()? && is_exists {
            let mut reader = BufReader::new(file);
            let new_list = Self::reader_to_list(&mut reader);

            let mut lock = self.list.write().await;
            new_list.iter().for_each(|num| {
                lock.insert(*num);
            })
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(MATCH_NOT_FOUND_FILE_PATH)?;

        let mut writer = BufWriter::new(file);
        let lock = self.list.read().await;

        lock.iter().for_each(|num| {
            let _ = writer.write(
                &format!("{},", num).as_bytes()
            );
        });

        let _ = writer.flush();

        Ok(())
    }
}
