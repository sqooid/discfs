
create table node (
    id integer primary key,
    name text,
    size integer,
    ctime float,
    atime float,
    parent integer,
    directory boolean not null,
    cloud_id text,
    foreign key(parent) references node(id)
); 

insert into node (id, name, parent, directory) values (1, null, null, true);