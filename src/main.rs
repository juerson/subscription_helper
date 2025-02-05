use futures::stream::{ self, StreamExt };
use hashbrown::HashSet as hashbrownHashset;
use regex::Regex;
use reqwest::Client;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::{
    collections::{ hash_map::DefaultHasher, HashMap, HashSet },
    fs::{ self, File },
    hash::{ Hash, Hasher },
    io::{ BufRead, BufReader },
    path::{ Path, PathBuf },
};

#[tokio::main]
async fn main() {
    let all_url = get_urls();
    if all_url.len() > 0 {
        let file = File::create("全部订阅地址.json").unwrap();
        serde_json::to_writer_pretty(&file, &all_url).unwrap();
        println!("全部订阅地址({}条)已经写入文件", all_url.len());
        // 抓取网页的内容，并比较数据、去重，获取唯一内容的url
        let url_map = extract_unique_urls(all_url.clone()).await;
        // 写入文件
        write_to_file(url_map);
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
                continue; // 跳过无法读取的行
            }
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

async fn extract_unique_urls(urls: Vec<String>) -> HashMap<String, Vec<String>> {
    let client = Client::new();

    let fetches = stream
        ::iter(
            urls.into_iter().map(|url| {
                let client = client.clone();
                async move {
                    match client.get(url.clone()).send().await {
                        Ok(response) =>
                            match response.text().await {
                                Ok(text) => Some((url, text)),
                                Err(_) => None,
                            }
                        Err(_) => None,
                    }
                }
            })
        )
        .buffer_unordered(10) // 并发执行的任务数量
        .collect::<Vec<_>>().await;

    let mut json_set = hashbrownHashset::new();
    let mut yaml_set = hashbrownHashset::new();
    let mut url_map: HashMap<String, Vec<String>> = HashMap::new();
    for fetch in fetches {
        if let Some((url, content)) = fetch {
            if
                content.trim().is_empty() ||
                content.contains("Package size exceeded the configured limit of 50 MB")
            {
                continue; // Skip empty content or package size exceeded
            }
            if let Ok(json_value) = serde_json::from_str::<JsonValue>(&content) {
                let json_string = serde_json::to_string(&json_value).unwrap();
                let mut hasher = DefaultHasher::new();
                json_string.hash(&mut hasher);
                let hash = hasher.finish();
                if json_set.insert(hash) {
                    if let Some(urls) = url_map.get_mut("json") {
                        if !urls.contains(&url.to_string()) {
                            urls.push(url.to_string());
                            // println!("Unique JSON URL: {}", url);
                        }
                    } else {
                        url_map.insert("json".to_string(), vec![url.to_string()]);
                        // println!("Unique JSON URL: {}", url);
                    }
                }
            } else if let Ok(yaml_value) = serde_yaml::from_str::<YamlValue>(&content) {
                let yaml_string = serde_yaml::to_string(&yaml_value).unwrap();
                let mut hasher = DefaultHasher::new();
                yaml_string.hash(&mut hasher);
                let hash = hasher.finish();
                if yaml_set.insert(hash) {
                    if let Some(urls) = url_map.get_mut("yaml") {
                        if !urls.contains(&url.to_string()) {
                            urls.push(url.to_string());
                            // println!("Unique yaml URL: {}", url);
                        }
                    } else {
                        url_map.insert("yaml".to_string(), vec![url.to_string()]);
                        // println!("Unique yaml URL: {}", url);
                    }
                }
            } else {
                // Handle other plain text or invalid formats if necessary
                // println!("Non-JSON/YAML content at {}: {}", url, content);
            }
        }
    }

    println!("{:#?}", url_map);
    println!("Unique JSON: {}", json_set.len());
    println!("Unique YAML: {}", yaml_set.len());

    url_map
}

fn write_to_file(url_map: HashMap<String, Vec<String>>) {
    let mut all_url: Vec<String> = Vec::new();
    for set in url_map.clone().values() {
        for url in set {
            all_url.push(url.clone());
        }
    }
    if all_url.len() > 0 {
        let file = File::create("订阅地址.json").unwrap();
        serde_json::to_writer_pretty(&file, &all_url).unwrap();
        println!("内容互不相同的{}条订阅地址已经写入文件", all_url.len());
    }
}
