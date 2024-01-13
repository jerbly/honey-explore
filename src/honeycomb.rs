use std::{
    collections::HashMap,
    env,
    fmt::{Display, Formatter},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct HoneyComb {
    pub api_key: String,
}
const URL: &str = "https://api.honeycomb.io/1/";
const HONEYCOMB_API_KEY: &str = "HONEYCOMB_API_KEY";

#[derive(Debug, Deserialize)]
pub struct Dataset {
    pub slug: String,
    pub last_written_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Column {
    pub id: String,
    pub key_name: String,
    pub r#type: String,
    pub description: String,
    pub hidden: bool,
    pub last_written: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct QueryResultLinks {
    query_url: String,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    links: QueryResultLinks,
}

#[derive(Debug, Deserialize)]
struct Query {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NameAndSlug {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Authorizations {
    pub api_key_access: HashMap<String, bool>,
    pub environment: NameAndSlug,
    pub team: NameAndSlug,
}

impl Authorizations {
    pub fn has_required_access(&self, access_types: &[&str]) -> bool {
        access_types
            .iter()
            .all(|access_type| *self.api_key_access.get(*access_type).unwrap_or(&false))
    }
}

impl Display for Authorizations {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut api_key_access = String::new();
        for (key, value) in &self.api_key_access {
            api_key_access.push_str(&format!("{}: {}\n", key, value));
        }
        write!(
            f,
            "api_key_access:\n{}\nenvironment: {}\nteam: {}",
            api_key_access, self.environment.name, self.team.name
        )
    }
}

impl HoneyComb {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            api_key: env::var(HONEYCOMB_API_KEY).context(format!(
                "Environment variable {} not found",
                HONEYCOMB_API_KEY
            ))?,
        })
    }

    async fn get<T>(&self, request: &str) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = reqwest::Client::new()
            .get(format!("{}{}", URL, request))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()
            .await?;
        let text: String = response.text().await?;

        match serde_json::from_str::<T>(&text) {
            Ok(t) => Ok(t),
            Err(e) => {
                eprintln!("Invalid JSON data: {}", text);
                Err(anyhow::anyhow!("Failed to parse JSON data: {}", e))
            }
        }
    }

    pub async fn list_authorizations(&self) -> anyhow::Result<Authorizations> {
        self.get("auth").await
    }
    pub async fn list_all_datasets(&self) -> anyhow::Result<Vec<Dataset>> {
        self.get("datasets").await
    }
    pub async fn list_all_columns(&self, dataset_slug: &str) -> anyhow::Result<Vec<Column>> {
        self.get(&format!("columns/{}", dataset_slug)).await
    }

    async fn post<T>(&self, request: &str, json: Value) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = reqwest::Client::new()
            .post(format!("{}{}", URL, request))
            .header("X-Honeycomb-Team", &self.api_key)
            .json(&json)
            .send()
            .await?;
        let text: String = response.text().await?;

        match serde_json::from_str::<T>(&text) {
            Ok(t) => Ok(t),
            Err(e) => {
                eprintln!("Invalid JSON data: {}", text);
                Err(anyhow::anyhow!("Failed to parse JSON data: {}", e))
            }
        }
    }

    pub async fn get_exists_query_url(
        &self,
        dataset_slug: &str,
        column_id: &str,
    ) -> anyhow::Result<String> {
        let query: Query = self
            .post(
                &format!("queries/{}", dataset_slug),
                serde_json::json!({
                    "breakdowns": [column_id],
                    "calculations": [{
                        "op": "COUNT"
                    }],
                    "filters": [{
                        "column": column_id,
                        "op": "exists",
                    }],
                    "time_range": 7200
                }),
            )
            .await?;

        let query_result: QueryResult = self
            .post(
                &format!("query_results/{}", dataset_slug),
                serde_json::json!({
                  "query_id": query.id,
                  "disable_series": false,
                  "limit": 10000
                }),
            )
            .await?;

        Ok(query_result.links.query_url)
    }
}
