/**
 * @module
 * Node's inputs and outputs are encoded as `IValue`.
 */

import { bs58, base64, web3, type Buffer } from "./deps.ts";

/**
 * JSON representation of `Value`.
 */
export interface IValue {
  S?: string;
  D?: string;
  I?: string;
  U?: string;
  I1?: string;
  U1?: string;
  F?: string;
  B?: boolean;
  N?: 0;
  B3?: string;
  B6?: string;
  BY?: string;
  A?: IValue[];
  M?: Record<string, IValue>;
}

export function isIValue(v: IValue): v is IValue {
  const keys = Object.keys(v);
  if (keys.length !== 1) return false;
  if (v.S !== undefined) return typeof v.S === "string";
  if (v.D !== undefined) return typeof v.D === "string";
  if (v.I !== undefined) return typeof v.I === "string";
  if (v.U !== undefined) return typeof v.U === "string";
  if (v.I1 !== undefined) return typeof v.I1 === "string";
  if (v.U1 !== undefined) return typeof v.U1 === "string";
  if (v.F !== undefined) return typeof v.F === "string";
  if (v.B !== undefined) return typeof v.B === "boolean";
  if (v.N !== undefined) return v.N === 0;
  if (v.B3 !== undefined) return typeof v.B3 === "string";
  if (v.B6 !== undefined) return typeof v.B6 === "string";
  if (v.BY !== undefined) return typeof v.BY === "string";
  if (v.A !== undefined) return Array.isArray(v.A) && v.A.every(isIValue);
  if (v.M !== undefined) return Object.values(v.M).every(isIValue);
  return false;
}

interface MaybePubkey {
  toBase58(): string;
  toBuffer(): Buffer;
}

interface MaybeKeypair {
  publicKey: MaybePubkey;
  secretKey: Uint8Array;
}

function maybePublicKey(x: MaybePubkey): x is web3.PublicKey {
  if (x == null) return false;
  return (
    typeof x.toBase58 === "function" &&
    typeof x.toBuffer === "function" &&
    x.toBuffer()?.byteLength === 32
  );
}

function maybeKeypair(x: MaybeKeypair): x is web3.Keypair {
  if (x == null) return false;
  return maybePublicKey(x.publicKey) && x.secretKey?.byteLength === 32;
}

function validateFloat(s: string) {
  if (!s.match(/^\-?[0-9]+(e[0-9]+)?(\.[0-9]+)?$/)) {
    throw new SyntaxError(`invalid number: ${s}`);
  }
}

function validateInt(s: string) {
  BigInt(s);
}

function validateUInt(s: string) {
  if (BigInt(s) < 0n) {
    throw new SyntaxError("number is negative");
  }
}

function validateBase58(s: string, length: number) {
  const bytes = bs58.decodeBase58(s);
  if (bytes.byteLength !== length) {
    throw new SyntaxError(`bytes length ${bytes.byteLength} != ${length}`);
  }
}

function validateBase64(s: string) {
  base64.decodeBase64(s);
}

export class Value implements IValue {
  S?: string;
  D?: string;
  I?: string;
  U?: string;
  I1?: string;
  U1?: string;
  F?: string;
  B?: boolean;
  N?: 0;
  B3?: string;
  B6?: string;
  BY?: string;
  A?: Value[];
  M?: Record<string, Value>;

  constructor(x?: any, customConvert?: (x: any) => Value | undefined) {
    if (x === undefined) {
      return Value.Null();
    }

    const value = Value.#inferFromJSType(x, customConvert);
    if (value === undefined) throw TypeError("undefined");
    return value;
  }

  toFlowValue(): Value {
    return this;
  }

  static #inferFromJSType(
    x: any,
    customConvert?: (x: any) => Value | undefined
  ): Value | undefined {
    if (x instanceof Value) {
      return x;
    }
    if (typeof x?.toFlowValue === "function") {
      return x.toFlowValue();
    }
    switch (typeof x) {
      case "function":
        return undefined;
      case "number":
        return Value.Decimal(x);
      case "boolean":
        return Value.Boolean(x);
      case "string":
        return Value.String(x);
      case "undefined":
        return Value.Null();
      case "bigint":
        if (x < BigInt(0)) {
          return Value.I128(x);
        } else {
          return Value.U128(x);
        }
      case "symbol":
        return Value.String(x.toString());
      case "object":
        if (x === null) {
          return Value.Null();
        }
        if (maybePublicKey(x)) {
          return Value.fromJSON({ B3: x.toBase58() });
        }
        if (maybeKeypair(x)) {
          return Value.fromJSON({
            B6: bs58.encodeBase58(
              new Uint8Array([...x.secretKey, ...x.publicKey.toBuffer()])
            ),
          });
        }
        if (typeof x.byteLength === "number") {
          switch (x.byteLength) {
            case 32:
              return Value.fromJSON({
                B3: bs58.encodeBase58(x),
              });
            case 64:
              return Value.fromJSON({
                B6: bs58.encodeBase58(x),
              });
            default:
              return Value.fromJSON({
                BY: base64.encodeBase64(x),
              });
          }
        }
        if (customConvert !== undefined) {
          const result = customConvert(x);
          if (result !== undefined) {
            return result;
          }
        }
        if (Object.prototype.isPrototypeOf.call(Array.prototype, x)) {
          return Value.fromJSON({
            A: Array.from(x)
              .map((x) => Value.#inferFromJSType(x, customConvert))
              .filter((x) => x !== undefined) as IValue[],
          });
        }
        return Value.fromJSON({
          M: Object.fromEntries(
            Object.entries(x)
              .map(([k, v]) => [k, Value.#inferFromJSType(v, customConvert)])
              .filter(([_k, v]) => v !== undefined)
          ),
        });
    }
  }

  public validate() {
    if (!isIValue(this)) throw SyntaxError("invalid JSON data");
    if (this.S !== undefined) return;
    if (this.D !== undefined) validateFloat(this.D);
    if (this.I !== undefined) validateInt(this.I);
    if (this.U !== undefined) validateUInt(this.U);
    if (this.I1 !== undefined) validateInt(this.I1);
    if (this.U1 !== undefined) validateUInt(this.U1);
    if (this.F !== undefined) validateFloat(this.F);
    if (this.B !== undefined) return;
    if (this.N !== undefined) return;
    if (this.B3 !== undefined) validateBase58(this.B3, 32);
    if (this.B6 !== undefined) validateBase58(this.B6, 64);
    if (this.BY !== undefined) validateBase64(this.BY);
    if (this.A !== undefined) this.A.forEach((v) => v.validate());
    if (this.M !== undefined)
      Object.values(this.M).forEach((v) => v.validate());
  }

  static #fromJSONUnchecked(obj: IValue): Value {
    if (obj instanceof Value) {
      return obj;
    }

    if (obj.A !== undefined) {
      obj.A = obj.A.map(Value.#fromJSONUnchecked);
    } else if (obj.M !== undefined) {
      obj.M = Object.fromEntries(
        Object.entries(obj.M).map(([k, v]) => [k, Value.#fromJSONUnchecked(v)])
      );
    }
    return Object.assign(Object.create(Value.prototype), obj);
  }

  /**
   * New Value from JSON data.
   */
  public static fromJSON(obj: IValue): Value {
    const value = Value.#fromJSONUnchecked(obj);
    value.validate();
    return value;
  }

  public static U64(x: string | number | bigint): Value {
    const i = BigInt(x);
    if (i < BigInt(0) || i > BigInt("18446744073709551615")) {
      throw new Error("value out of range");
    }
    return Value.#fromJSONUnchecked({ U: i.toString() });
  }

  public static I64(x: string | number | bigint): Value {
    const i = BigInt(x);
    if (
      i < BigInt("-9223372036854775808") ||
      i > BigInt("9223372036854775807")
    ) {
      throw new Error("value out of range");
    }
    return Value.#fromJSONUnchecked({ I: i.toString() });
  }

  public static String(x: string): Value {
    return Value.#fromJSONUnchecked({ S: x });
  }

  public static Float(x: number): Value {
    return Value.#fromJSONUnchecked({ F: x.toString() });
  }

  public static Decimal(x: number | string | bigint): Value {
    return Value.#fromJSONUnchecked({ D: x.toString() });
  }

  public static Null(): Value {
    return Value.#fromJSONUnchecked({ N: 0 });
  }

  public static Boolean(x: boolean): Value {
    return Value.#fromJSONUnchecked({ B: x });
  }

  public static U128(x: string | number | bigint): Value {
    const i = BigInt(x);
    if (
      i < BigInt(0) ||
      i > BigInt("340282366920938463463374607431768211455")
    ) {
      throw new Error("value out of range");
    }
    return Value.#fromJSONUnchecked({ U1: i.toString() });
  }

  public static I128(x: string | number | bigint): Value {
    const i = BigInt(x);
    if (
      i < BigInt("-170141183460469231731687303715884105728") ||
      i > BigInt("170141183460469231731687303715884105727")
    ) {
      throw new Error("value out of range");
    }
    return Value.#fromJSONUnchecked({ I1: i.toString() });
  }

  public static PublicKey(x: web3.PublicKeyInitData): Value {
    return Value.#fromJSONUnchecked({ B3: new web3.PublicKey(x).toBase58() });
  }

  public static Signature(
    x: string | Buffer | Uint8Array | ArrayBuffer
  ): Value {
    if (typeof x === "string") return Value.fromJSON({ B6: x });
    else return Value.#fromJSONUnchecked({ B6: bs58.encodeBase58(x) });
  }

  public static Keypair(
    x: string | Buffer | Uint8Array | ArrayBuffer | web3.Keypair
  ): Value {
    if (typeof x === "string") {
      return Value.fromJSON({ B6: x });
    }
    if ((x as web3.Keypair).secretKey !== undefined) {
      return Value.fromJSON({
        B6: bs58.encodeBase58((x as web3.Keypair).secretKey),
      });
    }
    return Value.fromJSON({
      B6: bs58.encodeBase58(x as any),
    });
  }

  public static Bytes(x: Buffer | Uint8Array | ArrayBuffer): Value {
    switch (x.byteLength) {
      case 32:
        return Value.#fromJSONUnchecked({ B3: bs58.encodeBase58(x) });
      case 64:
        return Value.#fromJSONUnchecked({ B6: bs58.encodeBase58(x) });
      default:
        return Value.#fromJSONUnchecked({ BY: base64.encodeBase64(x) });
    }
  }

  public asBool(): boolean | undefined {
    if (this.B !== undefined) return this.B;
    return undefined;
  }

  public asNumber(): number | undefined {
    if (this.U !== undefined) return parseInt(this.U);
    if (this.I !== undefined) return parseInt(this.I);
    if (this.F !== undefined) return parseFloat(this.F);
    if (this.U1 !== undefined) return parseInt(this.U1);
    if (this.I1 !== undefined) return parseInt(this.I1);
    return undefined;
  }

  public asBigInt(): bigint | undefined {
    if (this.U !== undefined) return BigInt(this.U);
    if (this.I !== undefined) return BigInt(this.I);
    if (this.U1 !== undefined) return BigInt(this.U1);
    if (this.I1 !== undefined) return BigInt(this.I1);
    return undefined;
  }

  public asString(): string | undefined {
    if (this.S !== undefined) return this.S;
    return undefined;
  }

  public asPubkey(): web3.PublicKey | undefined {
    const x = this.S ? bs58.decodeBase58(this.S) : this.asBytes();
    if (x !== undefined) {
      if (x.byteLength === 32) {
        return new web3.PublicKey(x);
      } else if (x.byteLength === 64) {
        return new web3.PublicKey(x.slice(32));
      }
    }
    return undefined;
  }

  public asKeypair(): web3.Keypair | undefined {
    if (this.S !== undefined) {
      return web3.Keypair.fromSecretKey(bs58.decodeBase58(this.S));
    }
    const x = this.asBytes();
    if (x !== undefined) {
      return web3.Keypair.fromSecretKey(x);
    }
    return undefined;
  }

  public asBytes(): Uint8Array | undefined {
    if (this.B3 !== undefined) return bs58.decodeBase58(this.B3);
    if (this.B6 !== undefined) return bs58.decodeBase58(this.B6);
    if (this.BY !== undefined) return base64.decodeBase64(this.BY);
    if (this.A !== undefined) {
      const x = this.A.map((v) => v.asNumber());
      if (
        x.every(
          (v) => v !== undefined && 0 <= v && v <= 255 && Number.isInteger(v)
        )
      ) {
        return new Uint8Array(x as number[]);
      }
    }
    return undefined;
  }

  public asArray(): Value[] | undefined {
    if (this.A !== undefined) return this.A;
    return undefined;
  }

  public asMap(): Record<string, Value> | undefined {
    if (this.M !== undefined) return this.M;
    return undefined;
  }

  public toJSObject(): any {
    if (this.S != undefined) return this.S;
    if (this.D != undefined) return parseFloat(this.D);
    if (this.I != undefined) return parseFloat(this.I);
    if (this.U != undefined) return parseFloat(this.U);
    if (this.I1 != undefined) return BigInt(this.I1);
    if (this.U1 != undefined) return BigInt(this.U1);
    if (this.F != undefined) return parseFloat(this.F);
    if (this.B != undefined) return this.B;
    if (this.N !== undefined) return null;
    if (this.B3 !== undefined) return bs58.decodeBase58(this.B3);
    if (this.B6 !== undefined) return bs58.decodeBase58(this.B6);
    if (this.BY !== undefined) return new TextEncoder().encode(atob(this.BY));
    if (this.A !== undefined) return this.A.map((x) => x.toJSObject());
    if (this.M !== undefined)
      return Object.fromEntries(
        Object.entries(this.M).map(([k, v]) => [k, v.toJSObject()])
      );
    throw "invalid value";
  }
}
