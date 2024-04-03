use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberTag {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberNote {
    pub note_id: String,
    pub created_at: String,
    pub created_by: String,
    pub note: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberMarketingPermission {
    pub marketing_permission_id: String,
    pub text: String,
    pub enabled: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub gmtoff: i64,
    pub dstoff: i64,
    pub country_code: String,
    pub timezone: String,
    pub region: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberStatsEcommerceData {
    pub total_revenue: f64,
    pub number_of_orders: u64,
    pub currency_code: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpMemberStats {
    pub avg_open_rate: f64,
    pub avg_click_rate: f64,
    pub ecommerce_data: Option<MailchimpMemberStatsEcommerceData>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MailchimpMember {
    pub id: String,
    pub email_address: String,
    pub unique_email_id: String,
    pub contact_id: String,
    pub full_name: String,
    pub web_id: u64,
    pub email_type: String,
    pub status: String,
    pub unsubscribe_reason: String,
    pub consents_to_one_to_one_messaging: bool,
    #[serde(default)]
    pub merge_fields: BTreeMap<String, String>,
    #[serde(default)]
    pub interests: BTreeMap<String, String>,
    pub stats: MailchimpMemberStats,
    pub ip_signup: String,
    pub timestamp_signup: String,
    pub ip_opt: String,
    pub timestamp_opt: String,
    pub member_rating: f32,
    pub last_changed: String,
    pub language: String,
    pub vip: bool,
    pub location: MailchimpMemberLocation,
    #[serde(default)]
    pub marketing_permissions: Vec<MailchimpMemberMarketingPermission>,
    pub last_note: Option<MailchimpMemberNote>,
    pub source: String,
    pub tags_count: u64,
    pub tags: Vec<MailchimpMemberTag>,
    pub list_id: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct MailchimpListResponse {
    pub members: Vec<MailchimpMember>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MailchimpError {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: Option<String>,
    pub istance: Option<String>,
}

impl Display for MailchimpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{} ({}): {}",
                self.title,
                self.status,
                self.detail.clone().unwrap_or_default()
            )
            .as_str(),
        )
    }
}
