/**
 * Test runner for Umbra node inline tests.
 *
 * Bun only discovers files matching *.test.ts / *.spec.ts.
 * Each Umbra .ts file has inline bun:test describe/test blocks,
 * so importing them here triggers test registration.
 */

import "./umbra_register.ts";
import "./umbra_deposit.ts";
import "./umbra_withdraw.ts";
import "./umbra_query_account.ts";
import "./umbra_query_balance.ts";
import "./umbra_create_utxo.ts";
import "./umbra_fetch_utxos.ts";
import "./umbra_claim_utxo.ts";
