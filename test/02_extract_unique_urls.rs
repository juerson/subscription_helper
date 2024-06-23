use futures::stream::{self, StreamExt};
use hashbrown::HashSet;
use reqwest::Client;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;

#[tokio::main]
async fn main() {
    let file_name = "全部订阅地址.json";
    let urls = read_json_file(file_name);
    // 抓取网页的内容，并比较数据、去重，获取唯一内容的url
    let url_map = extract_unique_urls(urls).await;
    // 写入文件
    write_to_json_file(url_map);
}

// 读取json，获取所有urls
fn read_json_file(file: &str) -> Vec<String> {
    // 打开文件并读取内容
    let mut file = File::open(file).expect("无法打开文件");
    let mut json_data = String::new();
    file.read_to_string(&mut json_data)
        .expect("无法读取文件内容");

    // 解析 JSON 数据为 Vec<String>
    let parsed_data: Vec<String> = serde_json::from_str(&json_data).expect("无法解析 JSON 数据");

    parsed_data
}

// 爬取所有urls，比较网页的内容、去重，返回唯一内容的url
async fn extract_unique_urls(urls: Vec<String>) -> HashMap<String, Vec<String>> {
    let client = Client::new();

    let fetches = stream::iter(urls.into_iter().map(|url| {
        let client = client.clone();
        async move {
            match client.get(url.clone()).send().await {
                Ok(response) => match response.text().await {
                    Ok(text) => Some((url, text)),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        }
    }))
    .buffer_unordered(10) // 并发执行的任务数量
    .collect::<Vec<_>>()
    .await;

    let mut json_set = HashSet::new();
    let mut yaml_set = HashSet::new();
    let mut url_map: HashMap<String, Vec<String>> = HashMap::new();
    for fetch in fetches {
        if let Some((url, content)) = fetch {
            if content.trim().is_empty()
                || content.contains("Package size exceeded the configured limit of 50 MB")
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
    println!("Unique JSON: {}", json_set.len());
    println!("Unique YAML: {}", yaml_set.len());
    println!("{:#?}", url_map);
    url_map
}

// 将结果写入json文件中
fn write_to_json_file(url_map: HashMap<String, Vec<String>>) {
    let mut all_url: Vec<String> = Vec::new();
    for set in url_map.clone().values() {
        for url in set {
            all_url.push(url.clone());
        }
    }
    if all_url.len() > 0 {
        let file = File::create("订阅地址.json").unwrap();
        serde_json::to_writer_pretty(&file, &all_url).unwrap();
        println!("{}条订阅地址已经写入文件", all_url.len());
    }
}
