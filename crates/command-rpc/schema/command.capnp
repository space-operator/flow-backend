@0xea2e5dc9f8697f6c;

struct Uuid {
    i0 @0 : UInt64;
    i1 @1 : UInt64;
}

struct Endpoints {
    flowServer @0 : Text;
    supabase @1 : Text;
    supabaseAnonKey @2 : Text;
}

struct User {
    id @0 : Uuid;
}

struct HttpClientConfig {
    timeoutInSecs @0 : UInt64;
    gzip @1 : Bool;
}

enum SolanaNet {
    devnet @0;
    testnet @1;
    mainnet @2;
}

struct SolanaClientConfig {
    url @0 : Text;
    cluster @1 : SolanaNet;
}

struct FlowSetContextData {
    flowOwner @0 : User;
    startedBy @1 : User;
    endpoints @2 : Endpoints;
    solana @3 : SolanaClientConfig;
    http @4 : HttpClientConfig;
}

struct Map(Key, Value) {
  entries @0 : List(Entry);
  struct Entry {
    key @0 : Key;
    value @1 : Value;
  }
}

struct FlowContextData {
    flowRunId @0 : Uuid;
    environment @1 : Map(Text, Text);
    set @2 : FlowSetContextData;
}

struct CommandContextData {
    nodeId @0 : Uuid;
    times @1 : UInt32;
    flow @2 : FlowContextData;
}
