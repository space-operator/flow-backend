grant usage on schema auth to flow_runner;

create table if not exists auth.passwords (
    user_id uuid not null,
    password varchar not null,
    primary key (user_id)
);

alter table auth.passwords add constraint "fk-user_id"
    foreign key (user_id) references auth.users (id)
    on delete cascade;

grant all on auth.passwords to flow_runner;
grant update on auth.users to flow_runner;

create or replace function auth.validate_user()
returns trigger as $$
declare
myrec record;
begin
    select * into myrec from public.pubkey_whitelists
    where pubkey = new.raw_user_meta_data->>'pub_key' and pubkey is not null;
    if not found then
        raise exception 'pubkey is not in whitelists, %', new.raw_user_meta_data->>'pub_key';
    end if;

    return new;
end;
$$ language plpgsql;

drop trigger if exists on_auth_check_whitelists on auth.users;
create trigger on_auth_check_whitelists
before insert on auth.users
for each row
execute procedure auth.validate_user();
