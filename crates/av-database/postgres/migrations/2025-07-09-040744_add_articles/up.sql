-- Your SQL goes here
create table articles
(
    hashid   Text primary key not null,
    sourceid int references sources (id) not null,
    category text      not null,
    title    text      not null,
    url      text      not null,
    summary  text      not null,
    banner   text      not null,
    author   int references authors (id) not null,
    ct       timestamp not null
);
