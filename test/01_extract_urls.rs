use regex::Regex;
use std::{
    collections::{ HashMap, HashSet },
    fs::{ self, File },
    io::{ BufRead, BufReader },
    path::{ Path, PathBuf },
};

fn main() {
    let all_url = get_urls();
    if all_url.len() > 0 {
        let file = File::create("全部订阅地址.json").unwrap();
        serde_json::to_writer_pretty(&file, &all_url).unwrap();
        println!("全部订阅地址({}条)已经写入文件", all_url.len());
    }
}

fn get_urls() -> Vec<String> {
    let current_dir = std::env::current_dir().unwrap();
    let bat_files = find_bat_files(&current_dir).unwrap();
    let mut url_map: HashMap<String, HashSet<String>> = HashMap::new();
    for file in bat_files {
        if let Some(first_folder) = get_first_folder(&current_dir, &file) {
            let urls = extract_urls(&file).unwrap();
            url_map.entry(first_folder).or_insert_with(HashSet::new).extend(urls);
        }
    }
    let mut all_url: Vec<String> = Vec::new();
    for set in url_map.clone().values() {
        for url in set {
            all_url.push(url.clone());
        }
    }
    all_url
}

fn find_bat_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut bat_files = Vec::new();
    let re = Regex::new(r"^ip_\d+\.bat$").unwrap();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            bat_files.extend(find_bat_files(&path)?);
        } else if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if re.is_match(filename) {
                bat_files.push(path);
            }
        }
    }

    Ok(bat_files)
}

fn extract_urls(file_path: &Path) -> std::io::Result<HashSet<String>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let url_re = Regex::new(r#"https?://[^\s\"\'<>]+"#).unwrap();
    let mut urls = HashSet::new();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                for url in url_re.find_iter(&line) {
                    urls.insert(url.as_str().to_string());
                }
            }
            Err(_) => {
                continue;
            } // 跳过无法读取的行
        }
    }

    Ok(urls)
}

fn get_first_folder(base_dir: &Path, file_path: &Path) -> Option<String> {
    file_path
        .strip_prefix(base_dir)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()
        .map(|s| s.to_string())
}
