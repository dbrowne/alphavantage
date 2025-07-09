-- Your SQL goes here
create table sources
(
    id          serial primary key,
    source_name text not null,
    domain      text not null
);

