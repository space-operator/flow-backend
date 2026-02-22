import { Schema } from "effect";

export const UpsertWalletBody = Schema.Struct({
  public_key: Schema.String,
  type: Schema.optional(Schema.String),
  name: Schema.optional(Schema.String),
  keypair: Schema.optional(Schema.String),
  user_id: Schema.optional(Schema.String),
});
export type UpsertWalletBody = typeof UpsertWalletBody.Type;

export const UpsertWalletResponseItem = Schema.Struct({
  id: Schema.Number,
  public_key: Schema.String,
});

export const UpsertWalletResponse = Schema.Array(UpsertWalletResponseItem);
export type UpsertWalletResponse = typeof UpsertWalletResponse.Type;
