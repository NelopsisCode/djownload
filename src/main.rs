use async_recursion::async_recursion;
use clap::Parser;
use google_youtube3::YouTube;
use google_youtube3::{Error, Result};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[derive(Parser)]
#[clap(version = "0.1", author = "Polytech Montpellier - DevOps")]
struct CLIOpts {
    /// Youtube Playlist ID
    #[clap(short, long, default_value = "PL0R2Ug2nH0zoRw5Wc_jclSUxNv1mFH1dW")]
    playlist_id: String,
}

// Get the list of songs from a youtube playlist url
async fn get_song_names_from_playlist_url(url: String) -> Result<Vec<Option<String>>> {
    // Read application secret from a file. Sometimes it's easier to compile it directly into
    // the binary. The clientsecret file contains JSON like `{"installed":{"client_id": ... }}`
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file named tokencache.json. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("tokencache.json")
        .build()
        .await
        .unwrap();

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    // Get the list of songs from the playlist
    let result = hub
        .playlist_items()
        .list(&vec!["snippet".into()])
        .playlist_id(&url)
        .max_results(50)
        .doit()
        .await;

    match result {
        #[warn(unused_variables)]
        Ok((response, playlist_item_list_response)) => {
            let mut song_names = Vec::new();
            extract_songs(&playlist_item_list_response, &mut song_names);

            if let Some(value) = get_all_songs_from_pagination_recursive(
                playlist_item_list_response,
                &mut song_names,
                hub,
                url,
            )
            .await
            {
                return value;
            }
            Ok(song_names)
        }
        Err(e) => Err(Error::from(e)),
    }
}

#[async_recursion]
async fn get_all_songs_from_pagination_recursive(
    playlist_item_list_response: google_youtube3::api::PlaylistItemListResponse,
    song_names: &mut Vec<Option<String>>,
    hub: YouTube,
    url: String,
) -> Option<Result<Vec<Option<String>>>> {
    // extract_songs(&playlist_item_list_response, song_names);

    while playlist_item_list_response.next_page_token.is_some() {
        let next_page_token = playlist_item_list_response
            .next_page_token
            .as_ref()
            .unwrap();

        println!("Next page token: {}", next_page_token);

        let result = hub
            .playlist_items()
            .list(&vec!["snippet".into()])
            .playlist_id(&url)
            .max_results(50)
            .page_token(&next_page_token)
            .doit()
            .await;

        match result {
            Ok((response, playlist_item_list_response)) => {
                // Get the song names from the playlist
                extract_songs(&playlist_item_list_response, song_names);
                return get_all_songs_from_pagination_recursive(
                    playlist_item_list_response,
                    song_names,
                    hub,
                    url,
                )
                .await;
            }
            Err(e) => {
                println!("Error: {:?}", e);
                return Some(Err(Error::from(e)));
            }
        }
    }
    None
}
// Get the list of songs from a youtube playlist url
fn extract_songs(
    playlist_item_list_response: &google_youtube3::api::PlaylistItemListResponse,
    song_names: &mut Vec<Option<String>>,
) {
    // Clone playlist_item_list_response
    let playlist_item_list_response = playlist_item_list_response.clone();
    let items = playlist_item_list_response.items.unwrap();

    for playlist_item in items {
        let snippet = playlist_item.snippet.unwrap();
        let title = snippet.title;
        song_names.push(title);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: CLIOpts = CLIOpts::parse();

    let song_names = get_song_names_from_playlist_url(opts.playlist_id).await;

    match song_names {
        Ok(song_names) => {
            for song_name in song_names {
                println!("{:?}", song_name);
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
            return Err(Error::from(e));
        }
    }

    Ok(())
}
