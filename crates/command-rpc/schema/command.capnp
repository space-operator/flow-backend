@0xea2e5dc9f8697f6c;

interface Context {
    data @0 () -> (data :Data);

    log @1 (level :Text, context :Text) -> (void :Void);
    sign @2 (input :Data) -> (output :Data);
    execute @3 (input :Data) -> (output :Data);
}

interface CommandWorker {
    initialize @0 (node_data :Data) -> (node :Node);
}

interface Node {
    run @0 (ctx :Context, inputs :Data) -> (outputs :Data);
}
