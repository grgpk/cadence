-- Add migration script here
CREATE TABLE availability (
    id              BIGSERIAL PRIMARY KEY,
    host_id         BIGINT NOT NULL REFERENCES hosts(id),
    weekday         INT NOT NULL,
    start_time      TIME NOT NULL,
    end_time        TIME NOT NULL,
    slot_minutes    INT NOT NULL
)