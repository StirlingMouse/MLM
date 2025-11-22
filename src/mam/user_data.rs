use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub unsat: Unsats,
    pub username: String,
    pub downloaded_bytes: f64,
    pub uploaded_bytes: f64,
    pub seedbonus: i64,
    pub wedges: u64,
    // pub classname: UserClass,
    // pub connectable: String,
    // pub country_code: Option<String>,
    // pub country_name: Option<String>,
    // pub created: u64,
    // pub downloaded: String,
    // pub duplicates: Duplicates,
    // #[serde(rename = "inactHnr")]
    // pub inact_hnr: InactHnr,
    // #[serde(rename = "inactSat")]
    // pub inact_sat: InactHnr,
    // #[serde(rename = "inactUnsat")]
    // pub inact_unsat: InactHnr,
    // pub ipv6_mac: bool,
    // pub ite: Ite,
    // pub last_access: Option<String>,
    // pub last_access_ago: Option<String>,
    // pub leeching: InactHnr,
    // pub partial: bool,
    // pub ratio: f64,
    // pub recently_deleted: u64,
    // pub reseed: Reseed,
    // #[serde(rename = "sSat")]
    // pub s_sat: InactHnr,
    // #[serde(rename = "seedHnr")]
    // pub seed_hnr: InactHnr,
    // #[serde(rename = "seedUnsat")]
    // pub seed_unsat: InactHnr,
    // pub uid: u64,
    // #[serde(rename = "upAct")]
    // pub up_act: InactHnr,
    // #[serde(rename = "upInact")]
    // pub up_inact: InactHnr,
    // pub update: u64,
    // pub uploaded: String,
    // pub username: String,
    // pub v6_connectable: bool,
    // pub vip_until: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unsats {
    pub count: u64,
    pub red: bool,
    pub size: Option<u64>,
    pub limit: u64,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Duplicates {
//     pub count: u64,
//     pub red: bool,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct InactHnr {
//     pub count: u64,
//     pub red: bool,
//     pub size: Option<u64>,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Ite {
//     pub count: u64,
//     pub latest: u64,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Reseed {
//     pub count: u64,
//     pub inactive: u64,
//     pub red: bool,
// }
