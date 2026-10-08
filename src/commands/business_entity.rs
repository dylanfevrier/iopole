//! Layout shared by the commands showing business entities, so that an entity
//! reads the same whether it comes from the operator or from the directory.

use std::mem;

use crate::iopole::{DirectoryEntryType, EntityIdentifier};
use crate::render::{MISSING, Table};

const IDENTIFIER_COLUMNS: [&str; 5] = [
    "TYPE",
    "IDENTIFIER",
    "NETWORK",
    "DIRECTORY ADDRESS",
    "AVAILABLE SINCE",
];

/// `Office (PRIMARY)`: the entity type, then its scope.
pub(crate) fn kind(entity_type: Option<&str>, scope: Option<&str>) -> String {
    let label = match entity_type.and_then(DirectoryEntryType::from_code) {
        Some(DirectoryEntryType::LegalUnit) => "Legal unit",
        Some(DirectoryEntryType::Office) => "Office",
        None => entity_type.unwrap_or(MISSING),
    };
    match scope {
        Some(scope) => format!("{label} ({scope})"),
        None => label.to_owned(),
    }
}

pub(crate) fn identifiers(identifiers: &[EntityIdentifier]) -> Table {
    let mut table = Table::new(&IDENTIFIER_COLUMNS);
    for identifier in identifiers {
        let value = identifier.value.as_deref().unwrap_or(MISSING);
        let mut kind = cell(identifier.identifier_type.as_deref());
        let mut id = match &identifier.scheme {
            Some(scheme) => format!("{scheme}:{value}"),
            None => value.to_owned(),
        };
        if identifier.network_registered.is_empty() {
            table.push([kind, id, MISSING.to_owned()]);
            continue;
        }
        for network in &identifier.network_registered {
            table.push([
                mem::take(&mut kind),
                mem::take(&mut id),
                cell(network.network_identifier.as_deref()),
                cell(network.directory_address.as_deref()),
                network.valid_from.clone().unwrap_or_default(),
            ]);
        }
    }
    table
}

fn cell(value: Option<&str>) -> String {
    value.unwrap_or(MISSING).to_owned()
}
