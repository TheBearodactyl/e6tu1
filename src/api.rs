use {
    crate::{
        app::DownloadProgress,
        models::{E6Post, E6PostResponse, E6PostsResponse},
    },
    color_eyre::eyre::{self, Result},
    futures::StreamExt,
    std::{
        fs::{self, File},
        io::Write,
    },
};

const USER_AGENT: &str = "E6TU1/1.0 (by bearodactyl on e621)";
const BASE_URL: &str = "https://e621.net";

pub struct E621Client {
    client: reqwest::Client,
}

impl E621Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn search_posts(&self, tags: &str) -> Result<Vec<E6Post>> {
        let url = format!(
            "{}/posts.json?tags={}&limit=50",
            BASE_URL,
            urlencoding::encode(tags)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(eyre::Error::msg(format!(
                "Failed to search posts: HTTP {}",
                response.status()
            )));
        }

        let posts_response: E6PostsResponse = response.json().await?;
        Ok(posts_response.posts)
    }

    pub async fn fetch_post(&self, post_id: &str) -> Result<E6Post> {
        let url = format!("{}/posts/{}.json", BASE_URL, post_id);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(eyre::Error::msg(format!(
                "Failed to fetch post: HTTP {}",
                response.status()
            )));
        }

        let post_response: E6PostResponse = response.json().await?;
        Ok(post_response.post)
    }

    pub async fn download_image_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;

        Ok(bytes.to_vec())
    }

    pub async fn download_post_to_file(
        &self,
        post: &E6Post,
        progress: &mut Option<DownloadProgress>,
    ) -> Result<()> {
        let image_url = post
            .file
            .url
            .as_ref()
            .ok_or_else(|| eyre::Error::msg("Post has no image URL"))?;

        fs::create_dir_all("downloads")?;

        let tags: Vec<String> = post
            .tags
            .general
            .iter()
            .chain(post.tags.artist.iter())
            .chain(post.tags.character.iter())
            .take(3)
            .map(|s| s.replace(['/', '\\'], "_"))
            .collect();

        let tag_string = if tags.is_empty() {
            "untagged".to_string()
        } else {
            tags.join("_")
        };

        let filename = format!(
            "downloads/{} - {} - {}.{}",
            post.id, tag_string, post.file.md5, post.file.ext
        );

        let response = self.client.get(image_url).send().await?;

        if !response.status().is_success() {
            eyre::bail!("Download failed with status: {}", response.status());
        }

        let total_size = response.content_length().unwrap_or(0);

        if let Some(p) = progress {
            p.total_bytes = total_size;
            p.downloaded_bytes = 0;
            p.message = format!(
                "Downloading {} ({:.2} MB)",
                filename,
                total_size as f64 / 1_048_576.0
            );
        }

        let mut file = File::create(&filename)?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            if let Some(p) = progress {
                p.downloaded_bytes = downloaded;
            }
        }

        file.flush()?;

        self.save_metadata(post, &filename)?;

        Ok(())
    }

    fn save_metadata(&self, post: &E6Post, base_filename: &str) -> Result<()> {
        let metadata_json = serde_json::to_string_pretty(post)?;

        #[cfg(target_os = "windows")]
        {
            let ads_path = format!("{}:metadata", base_filename);
            let mut ads_file = File::create(&ads_path)?;
            ads_file.write_all(metadata_json.as_bytes())?;
            ads_file.flush()?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let json_path = format!("{}.json", base_filename);
            let mut json_file = File::create(&json_path)?;
            json_file.write_all(metadata_json.as_bytes())?;
            json_file.flush()?;
        }

        Ok(())
    }
}
