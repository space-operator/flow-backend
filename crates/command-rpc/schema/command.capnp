@0xea2e5dc9f8697f6c;

interface CommandFactory {
    init @0 (nd: Data) -> (cmd: CommandTrait);

    allAvailables @1 () -> (availables: Data);
}

interface AddressBook {
    join @0 (direct_addresses: Data, relay_url: Text, availables: Data);

    leave @1 ();
}



interface CommandContext {
    data @0 () -> (data: Data);

    execute @1 (request: Data) -> (response: Data);

    getJwt @2 (user_id: Text) -> (access_token: Text);

    log @3 (logs: List(Log));

    struct Log {
        level @0 : LogLevel;
        content @1 : Text;

        enum LogLevel {
            trace @0;
            debug @1;
            info @2;
            warn @3;
            error @4;
        }
    }
}

interface CommandTrait {
    run @0 (ctx: CommandContext, inputs: Data) -> (output: Data);

    name @1 () -> (name: Text);

    inputs @2 () -> (inputs: Data);

    outputs @3 () -> (outputs: Data);

    instructionInfo @4 () -> (info: Data);

    permissions @5 () -> (permissions: Data);

    destroy @6 ();
}
