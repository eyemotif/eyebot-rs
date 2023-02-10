use irc::proto::message::Tag;
use std::collections::HashMap;

pub trait Tags
where
    Self: Sized,
{
    fn from_tags(
        tags: HashMap<String, Option<String>>,
    ) -> Option<(Self, HashMap<String, Option<String>>)>;
}

#[derive(Debug)]
pub struct CLEARCHATTags {
    pub room_id: String,
    pub target_user_id: Option<String>,
    /// timeout duration in seconds
    pub ban_duration: Option<u64>,
}

#[derive(Debug)]
pub struct CLEARMSGTags {
    pub login: String,
    pub target_msg_id: Option<String>,
}

// TODO: NOTICETags

#[derive(Debug)]
pub struct PRIVMSGTags {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub badges: HashMap<String, String>,
    pub bits: Option<u64>,
    /// original tag: mod
    pub is_mod: bool,
    pub subscriber: bool,
    pub vip: bool,
    // TODO: emotes
}

#[derive(Debug)]
pub struct USERNOTICETags {
    pub message_info: PRIVMSGTags,
    pub msg_id: String,
    pub sub: Option<NoticeSubTags>,
    pub raid: Option<NoticeRaidTags>,
}

#[derive(Debug)]
pub struct NoticeSubTags {
    /// orignial tags: msg-param-cumulative-months or msg-param-months
    pub months: u64,
    /// original tags: (msg-param-recipient-display-name, msg-param-recipient-id)
    pub gift_target: Option<(String, String)>,
}
#[derive(Debug)]
pub struct NoticeRaidTags {
    /// original tag: msg-param-displayName
    pub name: String,
    /// original tag: msg-param-viewerCount
    pub viewcount: u64,
}

pub fn tags<T: Tags>(raw_tags: &[Tag]) -> Option<T> {
    let tags_map = raw_tags
        .iter()
        .map(|Tag(k, v)| (k.clone(), v.clone()))
        .collect();
    T::from_tags(tags_map).map(|(tags, _)| tags)
}

impl CLEARCHATTags {
    pub fn is_chat_clear(&self) -> bool {
        self.target_user_id.is_none()
    }
    pub fn is_timeout(&self) -> bool {
        self.target_user_id.is_some() && self.ban_duration.is_some()
    }
    pub fn is_ban(&self) -> bool {
        self.target_user_id.is_some() && self.ban_duration.is_none()
    }
}

impl Tags for CLEARCHATTags {
    fn from_tags(
        mut tags: HashMap<String, Option<String>>,
    ) -> Option<(Self, HashMap<String, Option<String>>)> {
        let (Some(room_id), target_user_id, ban_duration) = (tags.remove("room-id"), tags.remove("target-user-id"), tags.remove("ban-duration")) else {
            return None;
        };
        Some((
            Self {
                room_id: room_id.expect("Tag always has a value"),
                target_user_id: target_user_id.map(|tag| tag.expect("Tag always has a value")),
                ban_duration: ban_duration.map(|tag| {
                    tag.expect("Tag always has a value")
                        .parse()
                        .expect("Tag is always a number")
                }),
            },
            tags,
        ))
    }
}

impl Tags for CLEARMSGTags {
    fn from_tags(
        mut tags: HashMap<String, Option<String>>,
    ) -> Option<(Self, HashMap<String, Option<String>>)> {
        let (Some(login), target_msg_id) = (tags.remove("login"), tags.remove("target-msg-id")) else {
            return None;
        };
        Some((
            Self {
                login: login.expect("Tag always has a value"),
                target_msg_id: target_msg_id.map(|tag| tag.expect("Tag always has a value")),
            },
            tags,
        ))
    }
}

impl Tags for PRIVMSGTags {
    fn from_tags(
        mut tags: HashMap<String, Option<String>>,
    ) -> Option<(Self, HashMap<String, Option<String>>)> {
        let (Some(id), Some(user_id), Some(display_name), Some(badges), bits, Some(is_mod), Some(subscriber), vip) = (tags.remove("id"), tags.remove("user-id"), tags.remove("display-name"), tags.remove("badges"), tags.remove("bits"), tags.remove("mod"), tags.remove("subscriber"), tags.remove("vip")) else {
            return None;
        };
        let badges = badges
            .expect("Tag always has a value")
            .split(',')
            .map(|badge| {
                badge
                    .split_once('/')
                    .expect("Badges are always formatted correctly")
            })
            .map(|(k, v)| (String::from(k), String::from(v)))
            .collect();
        Some((
            Self {
                id: id.expect("Tag always has a value"),
                user_id: user_id.expect("Tag always has a value"),
                display_name: display_name.expect("Tag always has a value"),
                badges,
                bits: bits.map(|tag| {
                    tag.expect("Tag always has a value")
                        .parse()
                        .expect("Tag is always a number")
                }),
                is_mod: is_mod.expect("Tag always has a value") == "1",
                subscriber: subscriber.expect("Tag always has a value") == "1",
                vip: vip.is_some(),
            },
            tags,
        ))
    }
}

impl Tags for USERNOTICETags {
    fn from_tags(
        tags: HashMap<String, Option<String>>,
    ) -> Option<(Self, HashMap<String, Option<String>>)> {
        let (message_info, mut tags) = PRIVMSGTags::from_tags(tags)?;
        let msg_id = tags.remove("msg-id")?.expect("Tag always has a value");

        let sub = {
            let (sub_months, gift_months, gift_target_name, gift_target_id) = (
                tags.remove("msg-param-cumulative-months"),
                tags.remove("msg-param-months"),
                tags.remove("msg-param-recipient-display-name"),
                tags.remove("msg-param-recipient-id"),
            );
            if let Some(sub_months) = sub_months {
                Some(NoticeSubTags {
                    months: sub_months
                        .expect("Tag always has a value")
                        .parse()
                        .expect("Tag is always a number"),
                    gift_target: None,
                })
            } else if let (Some(gift_months), Some(gift_target_name), Some(gift_target_id)) =
                (gift_months, gift_target_name, gift_target_id)
            {
                Some(NoticeSubTags {
                    months: gift_months
                        .expect("Tag always has a value")
                        .parse()
                        .expect("Tag is always a number"),
                    gift_target: Some((
                        gift_target_name.expect("Tag always has a value"),
                        gift_target_id.expect("Tag always has a value"),
                    )),
                })
            } else {
                None
            }
        };
        let raid = if let (Some(name), Some(viewcount)) = (
            tags.remove("msg-param-displayName"),
            tags.remove("msg-param-viewerCount"),
        ) {
            Some(NoticeRaidTags {
                name: name.expect("Tag always has a value"),
                viewcount: viewcount
                    .expect("Tag always has a value")
                    .parse()
                    .expect("Tag is always a number"),
            })
        } else {
            None
        };
        Some((
            Self {
                message_info,
                msg_id,
                sub,
                raid,
            },
            tags,
        ))
    }
}
