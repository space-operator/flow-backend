@0xea2e5dc9f8697f6c;

interface CommandFactory {
    init @0 (nd: Data) -> (cmd: CommandTrait);

    allAvailables @1 () -> (availables: Data);
}

interface AddressBook {
    join @0 (direct_addresses: Data, relay_url: Text, availables: Data, apikey: Text);

    leave @1 ();

    ping @2 ();
}

interface CommandContext {
    data @0 () -> (data: Data);

    execute @1 (request: Data) -> (response: Data);

    getJwt @2 (user_id: Text) -> (access_token: Text);

    log @3 (log: Data);

    requestSignature @4 (request: Data) -> (response: Data);
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
