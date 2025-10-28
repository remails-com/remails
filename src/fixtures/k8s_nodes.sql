INSERT INTO k8s_nodes (id, provider_id, hostname, ready)
VALUES ('46d37e7f-4b52-425c-a419-ed9488c83e47', 'aws:////i-0123456789abcdef0',
        'ip-10-0-0-1.us-west-2.compute.internal', false),
       ('44da8272-1b1d-4ab9-aa6b-27eff39c0510', 'k8s:////development-node',
        'mock-node-1', true);

INSERT INTO outbound_ips (id, ip, node_id)
VALUES ('e705e8d5-9a4d-471c-a226-fe558a54f2bc', '1.1.1.1', '46d37e7f-4b52-425c-a419-ed9488c83e47'),
       ('e4bcea34-5430-4ac7-8b4d-addf580873b1', '2.2.2.2', '46d37e7f-4b52-425c-a419-ed9488c83e47'),
       ('390ab7aa-ae59-4a1a-b5fb-3c85be3377d4', '127.0.0.1', '44da8272-1b1d-4ab9-aa6b-27eff39c0510')