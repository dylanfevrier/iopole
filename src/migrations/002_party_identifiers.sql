-- Parties are searched by the value of their electronic address, whatever its
-- scheme (0225:<SIREN>, 0002:<SIREN>, 0208:<BCE>...).
DROP INDEX invoice_seller_siren;
DROP INDEX invoice_buyer_siren;
ALTER TABLE invoice DROP COLUMN seller_siren;
ALTER TABLE invoice DROP COLUMN buyer_siren;

ALTER TABLE invoice ADD COLUMN seller_identifier TEXT;
ALTER TABLE invoice ADD COLUMN buyer_identifier TEXT;

UPDATE invoice SET
    seller_identifier = CASE WHEN instr(seller_address, ':') > 0
        THEN nullif(substr(seller_address, instr(seller_address, ':') + 1), '') END,
    buyer_identifier = CASE WHEN instr(buyer_address, ':') > 0
        THEN nullif(substr(buyer_address, instr(buyer_address, ':') + 1), '') END;

CREATE INDEX invoice_seller_identifier ON invoice (seller_identifier);
CREATE INDEX invoice_buyer_identifier ON invoice (buyer_identifier);
