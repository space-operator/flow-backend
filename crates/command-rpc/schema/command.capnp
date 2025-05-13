@0xea2e5dc9f8697f6c;

interface CommandContext {
    data @0 () -> (data: Data);
}

interface CommandFactory {
    init @0 (name: Text, nd: Data) -> (cmd: CommandTrait);

    allAvailables @1 () -> (availables: Data);
}

interface CommandTrait {
    run @0 (ctx: CommandContext, inputs: Data) -> (output: Data);
}
