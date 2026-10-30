-- Add migration script here
CREATE TABLE bids (
    id          BIGSERIAL PRIMARY KEY,
    username    TEXT NOT NULL,
    price       BIGINT NOT NULL,
    volume      BIGINT NOT NULL,
    seq         BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_bids_price_seq ON bids (price DESC, seq ASC) WHERE volume > 0;

CREATE TABLE allocations (
    username    TEXT PRIMARY KEY,
    volume      BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE supply_state (
    id       INT PRIMARY KEY DEFAULT 1,
    supply   BIGINT NOT NULL DEFAULT 0,
    buy_seq  BIGINT NOT NULL DEFAULT 0,
    CHECK (id = 1)
);
INSERT INTO supply_state (id, supply, buy_seq) VALUES (1, 0, 0);
