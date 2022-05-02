use crate::Database;
use aedron_patchouli_common::library::{LibraryConfig, LibraryKind};
use futures::{Stream, StreamExt};
use std::{
	collections::{HashSet, VecDeque},
	io,
	ops::DerefMut,
	path::{Path, PathBuf},
};

async fn parse_dir<P: AsRef<Path>>(
	queue: &mut VecDeque<PathBuf>,
	path: P,
) -> io::Result<Vec<PathBuf>> {
	use tokio::fs;

	let mut dir = fs::read_dir(path).await?;
	let mut files = Vec::new();
	while let Some(entry) = dir.next_entry().await? {
		let mut file_type = entry.file_type().await?;
		let mut file_path = entry.path();
		while file_type.is_symlink() {
			file_path = fs::read_link(file_path).await?;
			file_type = fs::metadata(&file_path).await?.file_type();
		}
		if file_type.is_file() {
			files.push(file_path);
		} else {
			queue.push_back(file_path);
		}
	}
	Ok(files)
}

fn walk_dirs<I: Iterator<Item = PathBuf>>(
	paths: I,
) -> impl 'static + Stream<Item = io::Result<PathBuf>> {
	use futures::stream;

	let queue: VecDeque<_> = paths.collect();
	let visited = HashSet::new();
	stream::unfold((visited, queue), |(mut visited, mut queue)| async {
		loop {
			let path = queue.pop_front()?;
			if !visited.insert(path.clone()) {
				continue;
			}
			let stream = match parse_dir(&mut queue, path).await {
				Ok(files) => stream::iter(files).map(Ok).left_stream(),
				Err(err) => stream::once(async { Err(err) }).right_stream(),
			};
			return Some((stream, (visited, queue)));
		}
	})
	.flatten()
}

pub(crate) async fn index_library(db: &Database, config: &LibraryConfig) {
	use once_cell::sync::Lazy;
	use std::{collections::HashMap, sync::Arc, time::Instant};
	use tokio::sync::{Mutex, RwLock};

	macro_rules! db_op {
		($op:expr) => {
			match $op {
				Ok(val) => val,
				Err(err) => {
					console_warn!("Database error", "{err}");
					return;
				}
			}
		};
	}

	// Lock library-associated Mutex to prevent other `index_library` tasks from running
	static LOCKS: Lazy<RwLock<HashMap<u64, Mutex<()>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
	if !LOCKS.read().await.contains_key(&config.id) {
		LOCKS.write().await.insert(config.id, Mutex::new(()));
	}
	let _lock = LOCKS.read().await;
	let _lock = _lock.get(&config.id).unwrap().lock().await;

	console_info!("Indexing library", "{}", config.name);

	let id = config.id as i64;
	let kind = config.kind;
	let instant = Instant::now();

	let temp_table_name = format!("temp_media_lib{}", config.id);
	let mut temp_db = db_op!(db.acquire().await).detach();
	db_op! {
		sqlx::query(&format!(
			"CREATE TEMPORARY TABLE {temp_table_name} AS SELECT path, 0 as found FROM media_{} WHERE library = {id}",
			format!("{kind:?}").to_ascii_lowercase()
		))
		.persistent(false)
		.execute(&mut temp_db)
		.await
	};
	let temp_db = Arc::new(Mutex::new(temp_db));

	walk_dirs(config.paths.iter().map(PathBuf::from))
		.filter_map(move |path| async move {
			let path = match path {
				Ok(path) => path,
				Err(err) => {
					return Some(Err(err));
				}
			};
			let ext = path.extension()?;
			kind.extensions()
				.iter()
				.any(|&filter_ext| ext.eq(filter_ext))
				.then(|| Ok(path))
		})
		.for_each_concurrent(db.size() as usize, |res| {
			let db = db.acquire();
			let temp_table_name = temp_table_name.clone();
			let temp_db = Arc::clone(&temp_db);
			async move {
				match res {
					Ok(path) => {
						let path_s = match path.to_str() {
							Some(path) => path,
							None => {
								console_warn!("Non-Unicode character in", "{}", path.to_string_lossy());
								return;
							}
						};
						let title = path.file_name().and_then(|s| s.to_str()).unwrap();
						let mut db = db_op!(db.await);
						if let Err(err) = match kind {
							LibraryKind::Image => sqlx::query!(
								"INSERT OR IGNORE INTO media_image (library, path, title) VALUES (?, ?, ?)",
								id,
								path_s,
								title
							),
							LibraryKind::Music => sqlx::query!(
								"INSERT OR IGNORE INTO media_music (library, path, title) VALUES (?, ?, ?)",
								id,
								path_s,
								title
							),
							_ => todo!(),
						}
						.persistent(false)
						.execute(&mut db)
						.await
						{
							console_warn!("Database error", "{err}");
						}
						db_op! {
							sqlx::query(&format!("UPDATE OR IGNORE temp.{temp_table_name} SET found = 1 WHERE path = {path_s:?}"))
							.persistent(false)
							.execute(temp_db.lock().await.deref_mut())
							.await
						};
					}
					Err(err) => {
						console_warn!("IO error", "{err}");
					}
				}
			}
		})
		.await;

	let mut temp_db = temp_db.lock().await;
	db_op! {
		sqlx::query(&format!(
			"DELETE FROM media_{} WHERE library = {id} AND path IN (SELECT path FROM temp.{temp_table_name} WHERE found = 0)",
			format!("{kind:?}").to_ascii_lowercase()
		))
		.persistent(false)
		.execute(temp_db.deref_mut())
		.await
	};
	db_op! {
		sqlx::query(&format!("DROP TABLE temp.{temp_table_name}"))
		.persistent(false)
		.execute(temp_db.deref_mut())
		.await
	};
	drop(temp_db);

	let count: i64 = db_op! {
		sqlx::query_scalar(&format!(
			"SELECT count(*) FROM media_{} WHERE library = {id}",
			format!("{kind:?}").to_ascii_lowercase()
		))
		.persistent(false)
		.fetch_one(&mut db_op!(db.acquire().await))
		.await
	};
	console_log!(
		"Indexed library",
		"{} (found {} media in {:.2}s)",
		config.name,
		count,
		instant.elapsed().as_secs_f32()
	);
}
