use actix_web::{http, web::{Data, Path}, HttpRequest, HttpResponse};
use bigdecimal::BigDecimal;
use serde_json::json;
use sqlx::PgPool;
use tracing::Span;

use crate::routes::movies::types::ImdbMovieDetails;

use super::{map_movie_bd_genre_code_with_value, Source};
// https://trakt.tv
// https://trakt.docs.apiary.io/#introduction/standard-media-objects
pub async fn get_movie_info(
    path: Path<(String, Source)>,
    connection: Data<PgPool>,
    req: HttpRequest,
    // query_span: Span,
) -> HttpResponse {
    let (movie_id, source_provider) = path.into_inner();
    // let query_span = tracing::Span::new(meta, values)

    if source_provider == Source::YTS {

        let movie_id: u32 = match movie_id.parse() {
            Ok(id) => id,
            Err(_) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "Invalid movie ID format"
                }));
            }
        };

        let movie_details = match yts_api::MovieDetails::new(movie_id)
            .with_cast(true)
            .with_images(true)
            .execute()
            .await
        {
            Ok(mut res) => {
                res.movie.yt_trailer_code = format!(
                    "https://www.youtube.com/watch?v={}",
                    res.movie.yt_trailer_code
                );
                res
            }
            Err(err) => {
                return HttpResponse::BadRequest().json(json!({
                  "error": err.to_string()
                }));
            }
        };
        let client = reqwest::Client::new();
        let similar_movies_suggestions = match client
            .get(format!(
                "https://yts.mx/api/v2/movie_suggestions.json?movie_id={}",
                movie_id
            ))
            .send()
            .await
        {
            Ok(res) => {
                tracing::info!("Got YTS movie recommendations");
                res.json::<serde_json::Value>().await
            }
            Err(err) => {
                tracing::error!("YTS movie recommendations Error");
                return HttpResponse::BadRequest().json(json!({
                    "error": err.to_string()
                }));
            }
        };
        match similar_movies_suggestions {
            Ok(res) => {
                return HttpResponse::Ok().json(json!({
                    "data": movie_details,
                    "movie_suggestions" : res
                }));
            }
            Err(_) => {
                return HttpResponse::Ok().json(json!({
                    "data": movie_details,
                }));
            }
        }
    } else if source_provider == Source::MovieDb {
        let client = reqwest::Client::new();
        // let res_body;
        
        let m = sqlx::query!(r#"
        Select * From imdb_movie_details Where id = $1
        "#, movie_id)
        .fetch_one(connection.get_ref())
        // .instrument(query_span.clone())
        .await;
    
    let movie = match m {
        Ok(val) => {
            val
        }
        Err(err) =>{
            tracing::error!("ERR RUERYING THE MOVIE : {}", err);
            return HttpResponse::BadRequest().finish();
        }
    };
    
    let imdb_movie_details = ImdbMovieDetails {
        id: movie.id,
        primary_title: movie.primary_title.unwrap_or_default(),
        original_title: movie.original_title.unwrap_or_default(),
        source_type: movie.source_type.unwrap_or_default(),
        genres: movie.genres.unwrap_or_default(),
        is_adult: movie.is_adult.unwrap_or_default(),
        start_year: movie.start_year.unwrap_or_default(),
        end_year: movie.end_year.unwrap_or_default(),
        runtime_minutes: movie.runtime_minutes.unwrap_or_default(),
        average_rating: movie.average_rating.unwrap_or_default().to_string(),
        num_votes: movie.num_votes.unwrap_or_default(),
        description: movie.description.unwrap_or_default(),
        primary_image: movie.primary_image.unwrap_or_default(),
        content_rating: movie.content_rating.unwrap_or_default(),
        release_date: movie.release_date,
        interests: movie.interests.unwrap_or_default(),
        countries_of_origin: movie.countries_of_origin.unwrap_or_default(),
        external_links: movie.external_links.unwrap_or_default(),
        spoken_languages: movie.spoken_languages.unwrap_or_default(),
        filming_locations: movie.filming_locations.unwrap_or_default(),
        directors: movie.directors.unwrap_or_default(),
        writers: movie.writers.unwrap_or_default(),
        cast: movie.cast.unwrap_or_default(),
        budget: movie.budget.unwrap_or_default(),
        gross_world_wide: movie.gross_world_wide.unwrap_or_default(),
        torrents: movie.torrents.unwrap_or_default(),
    };
        tracing::info!("THE QUERIED MOVIE: {:#?}", imdb_movie_details);

        return HttpResponse::Ok().json(imdb_movie_details);
    }
    HttpResponse::BadRequest().finish()
}
