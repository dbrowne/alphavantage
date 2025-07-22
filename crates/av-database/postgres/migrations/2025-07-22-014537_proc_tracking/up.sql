-- Your SQL goes here
create table proctypes (
                           id   serial primary key,
                           name text not null unique
);

create table states (
                        id   serial primary key,
                        name text not null unique
);

-- Insert default states
insert into states (name) values
                              ('started'),
                              ('completed'),
                              ('failed'),
                              ('cancelled');

create table procstates (
                            spid       serial primary key,
                            proc_id    integer references proctypes(id),
                            start_time timestamp not null,
                            end_state  integer references states(id),
                            end_time   timestamp
);

create index idx_procstates_proc_id on procstates (proc_id);
create index idx_procstates_start_time on procstates (start_time desc);