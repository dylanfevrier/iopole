mod auth;
mod business_entity;
mod french_directory;
mod http;
mod model;
mod query;
mod status_history;
mod uuid_bound;

#[cfg(test)]
pub(crate) mod testing;

pub(crate) use business_entity::{BusinessEntity, EntityIdentifier};
pub(crate) use french_directory::{
    DirectoryEntry, DirectoryEntryType, DirectoryPage, DirectoryQuery, PostalAddress,
};
pub(crate) use http::{Endpoints, HttpClient};
pub(crate) use query::SearchQuery;
pub(crate) use status_history::{StatusEvent, StatusParty, buyer, seller};
pub(crate) use uuid_bound::UuidBound;

use crate::Result;
use crate::invoice::Invoice;

pub(crate) const MAX_PAGE_LIMIT: u32 = 200;
pub(crate) const MAX_OFFSET: u32 = 1000;

pub(crate) trait InvoiceSearch {
    fn search(&mut self, query: &SearchQuery, offset: u32, limit: u32) -> Result<Vec<Invoice>>;
}

pub(crate) trait BusinessEntityDirectory {
    fn find(&mut self, identifier_value: &str) -> Result<Vec<BusinessEntity>>;
}

pub(crate) trait InvoiceStatuses {
    /// `None` when Iopole does not know the invoice.
    fn status_history(&mut self, invoice_id: &str) -> Result<Option<Vec<StatusEvent>>>;
}

pub(crate) trait FrenchDirectory {
    fn search_french(
        &mut self,
        query: &DirectoryQuery,
        offset: u32,
        limit: u32,
    ) -> Result<DirectoryPage>;
}

impl<D: FrenchDirectory> FrenchDirectory for &mut D {
    fn search_french(
        &mut self,
        query: &DirectoryQuery,
        offset: u32,
        limit: u32,
    ) -> Result<DirectoryPage> {
        (**self).search_french(query, offset, limit)
    }
}
