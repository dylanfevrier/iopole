CREATE TABLE invoice (
    invoice_id        TEXT PRIMARY KEY,
    direction         TEXT NOT NULL,
    state             TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    last_status_code  TEXT,
    last_status_state TEXT,
    last_status_date  TEXT,
    number            TEXT,              -- businessData.invoiceId, the issuer's number
    amount            TEXT,
    seller_name       TEXT,
    seller_address    TEXT,
    seller_siren      TEXT,
    seller_country    TEXT,
    buyer_name        TEXT,
    buyer_address     TEXT,
    buyer_siren       TEXT,
    buyer_country     TEXT,
    raw               TEXT NOT NULL,     -- without paymentMeans
    synced_at         TEXT NOT NULL
) STRICT;

CREATE INDEX invoice_buyer_siren  ON invoice (buyer_siren);
CREATE INDEX invoice_seller_siren ON invoice (seller_siren);
CREATE INDEX invoice_state        ON invoice (direction, state);
CREATE INDEX invoice_created_at   ON invoice (created_at);

-- Only `complete` ranges are skipped by later syncs.
CREATE TABLE sync_range (
    direction TEXT NOT NULL,
    from_id   TEXT NOT NULL,             -- inclusive
    to_id     TEXT NOT NULL,             -- exclusive
    from_at   TEXT NOT NULL,
    to_at     TEXT NOT NULL,
    status    TEXT NOT NULL CHECK (status IN ('complete', 'open', 'truncated')),
    fetched   INTEGER NOT NULL,
    synced_at TEXT NOT NULL,
    PRIMARY KEY (direction, from_id, to_id)
) STRICT;
