use dotenvy::dotenv;
use std::cmp::Ordering;
use std::env;
use std::error::Error;
use std::{thread, time::Duration};

use twitter_v2::authorization::BearerToken;
use twitter_v2::id::NumericId;
use twitter_v2::query::{TweetField, UserField};
use twitter_v2::{Tweet, TwitterApi};

use teloxide::prelude::*;
use teloxide::{types::ParseMode::MarkdownV2, utils::markdown::*};

struct User {
    name: String,
    id: NumericId,
}

fn build_query_of_tweets_from_multiple_users(users: &Vec<User>) -> String {
    let strings: Vec<String> = users
        .iter()
        .map(|u| format!("from:{}", u.id.as_u64().to_string()))
        .collect();
    strings.join(" OR ")
}

fn get_name_from_id(id: NumericId, users: &Vec<User>) -> &str {
    for u in users {
        if u.id == id {
            return &u.name;
        }
    }
    panic!("User {} not found.", id);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    pretty_env_logger::init();
    let bot = Bot::new(
        env::var("CHIRPEEP_BOT_TOKEN").expect("Environment variable `CHIRPEEP_BOT_TOKEN` not set."),
    )
    .auto_send();
    let chat = ChatId(-1001576907774);
    println!("机器猫猫开始运行了喵～");
    bot.send_message(chat, bold("机器猫猫开始运行了喵～"))
        .parse_mode(MarkdownV2)
        .await?;

    let users = vec![
        User {
            name: String::from("猫猫"),
            id: NumericId::new(758295043337617408),
        },
        User {
            name: String::from("红豆豆"),
            id: NumericId::new(1248118040803209216),
        },
        User {
            name: format!("{}猫猫", underline("世界")),
            id: NumericId::new(1464488463831556102),
        },
        User {
            name: "喵喵".into(),
            id: 711869340455309312.into(),
        },
    ];
    let q = &build_query_of_tweets_from_multiple_users(&users);

    let mut last_tweetid = NumericId::new(1590199074457112044);
    let auth = BearerToken::new(
        env::var("TWITTER_API_BEARER").expect("Environment variable `TWITTER_API_BEARER` not set."),
    );
    let api = TwitterApi::new(auth);

    loop {
        println!("Now fetching tweets…");
        let maybe_tweets: Option<Vec<Tweet>> = api
            .get_tweets_search_recent(q)
            .since_id(last_tweetid)
            .tweet_fields([
                TweetField::Entities,
                TweetField::ReferencedTweets,
                TweetField::AuthorId,
            ])
            .user_fields([UserField::Name, UserField::Username, UserField::Id])
            .send()
            .await?
            .into_data();
        if maybe_tweets.is_none() {
            thread::sleep(Duration::from_secs(60 * 20));
            continue;
        }
        let tweets = maybe_tweets.unwrap();
        println!("There are {} tweets", tweets.len());
        println!("{:?}", tweets);
        let last_tweetid_this_round = last_tweetid.clone();
        for t in tweets.iter().rev() {
            match t.id.cmp(&last_tweetid_this_round) {
                Ordering::Greater => {
                    last_tweetid = last_tweetid.max(t.id);
                    let replied_to_text = match t.in_reply_to_user_id {
                        Some(id) => {
                            match api
                                .get_tweet(id)
                                .tweet_fields([
                                    TweetField::Entities,
                                    TweetField::ReferencedTweets,
                                    TweetField::AuthorId,
                                ])
                                .user_fields([UserField::Name, UserField::Username, UserField::Id])
                                .send()
                                .await?
                                .into_data()
                            {
                                Some(t) => format!("\nIn reply to:\n{}", t.text),
                                None => String::from("[你无权查看]"),
                            }
                        }
                        None => String::from(""),
                    };
                    if t.text.contains("質問箱") {
                        continue;
                    }
                    let tweet_link = escape_link_url(&format!(
                        "https://fxtwitter.com/_/status/{}",
                        t.id.as_u64()
                    ));
                    let text = format!(
                        "{} just {}:\n{}\n{}",
                        get_name_from_id(t.author_id.unwrap(), &users), // user screen name
                        link(
                            &tweet_link,
                            if t.text.starts_with("RT @") {
                                "retweeted"
                            } else {
                                "tweeted"
                            }, // tweeted/retweeted
                        ), // <a>'
                        escape(&t.text),
                        escape(&replied_to_text)
                    );
                    println!("Message to be sent: {}", text);
                    match bot.send_message(chat, text).parse_mode(MarkdownV2).await {
                        Err(msg) => {
                            println!("Error on sending to the group: {:?}", msg);
                        }
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_secs(60 * 17));
    }
}
