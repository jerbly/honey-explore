use std::env;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct HoneyComb {
    pub api_key: String,
}
const URL: &str = "https://api.honeycomb.io/1/";
const HONEYCOMB_API_KEY: &str = "HONEYCOMB_API_KEY";

#[derive(Debug, Deserialize)]
pub struct Dataset {
    pub slug: String,
    pub last_written_at: DateTime<Utc>,
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

impl HoneyComb {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            api_key: env::var(HONEYCOMB_API_KEY).context(format!(
                "Environment variable {} not found",
                HONEYCOMB_API_KEY
            ))?,
        })
    }
    pub async fn list_all_datasets(&self) -> anyhow::Result<Vec<Dataset>> {
        let response = reqwest::Client::new()
            .get(format!("{}datasets", URL))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()
            .await?
            .json::<Vec<Dataset>>()
            .await?;
        Ok(response)
    }
    pub async fn list_all_columns(&self, dataset_slug: &str) -> anyhow::Result<Vec<Column>> {
        let response = reqwest::Client::new()
            .get(format!("{}columns/{}", URL, dataset_slug))
            .header("X-Honeycomb-Team", &self.api_key)
            .send()
            .await?
            .json::<Vec<Column>>()
            .await?;
        Ok(response)
    }

    pub async fn get_exists_query_url(
        &self,
        dataset_slug: &str,
        column_id: &str,
    ) -> anyhow::Result<String> {
        let response = reqwest::Client::new()
            .post(format!("{}queries/{}", URL, dataset_slug))
            .header("X-Honeycomb-Team", &self.api_key)
            .json(&serde_json::json!({
                "breakdowns": [
                    column_id
                ],
                "calculations": [{
                    "op": "COUNT"
                  }],
                "filters": [
                    {
                        "column": column_id,
                        "op": "exists",
                    }
                ],
                "time_range": 7200
            }))
            .send()
            .await?
            .json::<Query>()
            .await?;

        let query_id = response.id;

        let response = reqwest::Client::new()
            .post(format!("{}query_results/{}", URL, dataset_slug))
            .header("X-Honeycomb-Team", &self.api_key)
            .json(&serde_json::json!({
              "query_id": query_id,
              "disable_series": false,
              "limit": 10000
            }))
            .send()
            .await?
            .json::<QueryResult>()
            .await?;

        Ok(response.links.query_url)
    }
}
