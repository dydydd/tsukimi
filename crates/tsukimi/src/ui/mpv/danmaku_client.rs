use std::{
    ops::Deref,
    sync::OnceLock,
};

use dandanapi_client::*;
use mutsumi::Danmaku;

use crate::ui::mpv::danmaku::DanmakuExt;

// Allow overriding the dandanplay credentials at build time so you can use your own
// AppId/AppSecret instead of the shared one. Set DANDANPLAY_APPID and
// DANDANPLAY_APPSECRET when building (e.g. `DANDANPLAY_APPID=xxx DANDANPLAY_APPSECRET=yyy cargo build`).
const X_APPID: &str = match option_env!("DANDANPLAY_APPID") {
    Some(v) => v,
    None => "e9imrhcexn",
};

fn app_secret() -> String {
    match option_env!("DANDANPLAY_APPSECRET") {
        Some(secret) if !secret.is_empty() => secret.to_string(),
        _ => yzlm::yzlm(),
    }
}

#[derive(Clone)]
pub struct DanmakuClient(DanDanClient);

impl DanmakuClient {
    pub fn instance() -> Option<Self> {
        static CLIENT: OnceLock<Option<DanmakuClient>> = OnceLock::new();
        CLIENT
            .get_or_init(|| {
                if DanDanClient::init(X_APPID.to_string(), app_secret()).is_err() {
                    tracing::warn!("DanDanClient already initialized, using existing instance...");
                }
                Some(Self(DanDanClient::instance()))
            })
            .clone()
    }
}

impl Deref for DanmakuClient {
    type Target = DanDanClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DanmakuClient {
    pub async fn search_anime_details(
        &self, keyword: String, anime_type: AnimeType,
    ) -> anyhow::Result<Vec<SearchAnimeDetails>> {
        Ok(self
            .route(SearchSearchAnime {
                params: SearchSearchAnimeParams {
                    keyword,
                    r#type: anime_type,
                    v2: true,
                },
            })
            .await?
            .animes
            .unwrap_or_default())
    }

    pub async fn search_animes(
        &self, params: SearchSearchEpisodesParams,
    ) -> anyhow::Result<Vec<SearchEpisodesAnime>> {
        Ok(self
            .route(SearchSearchEpisodes { params })
            .await?
            .animes
            .unwrap_or_default())
    }

    pub async fn search_episode(
        &self, params: SearchSearchEpisodesParams,
    ) -> anyhow::Result<Option<(i64, String)>> {
        let anime = self.search_animes(params).await?;

        Ok(Self::first_episode_match(anime))
    }

    fn first_episode_match(animes: Vec<SearchEpisodesAnime>) -> Option<(i64, String)> {
        animes.into_iter().find_map(|anime| {
            let anime_title = anime.anime_title.unwrap_or_default();
            anime
                .episodes
                .unwrap_or_default()
                .into_iter()
                .find_map(|episode| {
                    let episode_id = episode.episode_id?;
                    let episode_title = episode.episode_title.unwrap_or_default();
                    let item_name = match (episode_title.is_empty(), anime_title.is_empty()) {
                        (false, false) => format!("{episode_title} - {anime_title}"),
                        (false, true) => episode_title,
                        (true, false) => anime_title.clone(),
                        (true, true) => String::new(),
                    };
                    Some((episode_id, item_name))
                })
        })
    }

    pub async fn get_comments(&self, episode_id: i64) -> anyhow::Result<Option<Vec<Danmaku>>> {
        let response = self
            .route(CommentGetComment {
                episode_id,
                params: CommentGetCommentParams {
                    from: 0,
                    with_related: true,
                    ch_convert: 0,
                },
            })
            .await?;

        let Some(comments) = response.comments else {
            return Ok(None);
        };

        let danmaku = comments
            .into_iter()
            .map(DanmakuExt::into_danmaku)
            .filter(|danmaku| !danmaku.content.is_empty())
            .collect::<Vec<_>>();

        Ok((!danmaku.is_empty()).then_some(danmaku))
    }
}
