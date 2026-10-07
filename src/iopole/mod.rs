mod auth;
mod http;
mod model;
mod query;
mod uuid_bound;

#[cfg(test)]
pub(crate) mod testing;

pub(crate) use http::{Endpoints, HttpClient};
pub(crate) use query::SearchQuery;
pub(crate) use uuid_bound::UuidBound;

use crate::Result;
use crate::invoice::Invoice;

pub(crate) const MAX_PAGE_LIMIT: u32 = 200;
pub(crate) const MAX_OFFSET: u32 = 1000;

pub(crate) trait InvoiceSearch {
    fn search(&mut self, query: &SearchQuery, offset: u32, limit: u32) -> Result<Vec<Invoice>>;
}
