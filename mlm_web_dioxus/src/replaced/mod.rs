mod components;
mod server_fns;
mod types;

pub use components::ReplacedPage;
pub use server_fns::{apply_replaced_action, get_replaced_data};
pub use types::{
    ReplacedBulkAction, ReplacedData, ReplacedMeta, ReplacedPageColumns, ReplacedPageFilter,
    ReplacedPageSort, ReplacedPairRow, ReplacedRow,
};
