CREATE TABLE k8s_nodes
(
    id          uuid NOT NULL PRIMARY KEY,
    provider_id text NOT NULL UNIQUE,
    hostname    text NOT NULL UNIQUE,
    ready       bool NOT NULL default false
);

CREATE TABLE outbound_ips
(
    id      uuid NOT NULL PRIMARY KEY,
    ip      inet NOT NULL UNIQUE,
    node_id uuid REFERENCES k8s_nodes (id) ON DELETE SET NULL
);

ALTER TABLE messages
    ADD COLUMN outbound_ip inet;