import * as lib from 'jsr:@space-operator/flow-lib@0.10.0';
import { createUmi } from 'npm:@metaplex-foundation/umi-bundle-defaults@0.9.2';
import { publicKey } from 'npm:@metaplex-foundation/umi@0.9.2';
import { dasApi } from 'npm:@metaplex-foundation/digital-asset-standard-api@1.0.4';
import { das } from 'npm:@metaplex-foundation/mpl-core-das@0.0.3';

interface Inputs {
  input_one: boolean;
  input_two: boolean;
  input_three: boolean;
}

export default class Playground extends lib.BaseCommand {
  async run(ctx: lib.Context, params: Inputs): Promise<Record<string, any>> {
    const umi = createUmi('https://eran-eafb8u-fast-mainnet.helius-rpc.com');
    umi.use(dasApi());

    const collection = publicKey(
      '5LCDufGQwVqeTUKkiUC6YEwo3C8YSpibZn4wLXGh33xg'
    );

    const response = await das.getAssetsByCollection(umi, {
      collection,
      limit: 10,
      page: 1,
    });

    const bigIntReplacer = (key: string, value: any) => {
      if (typeof value === 'bigint') {
        return value.toString();
      }
      return value;
    };

    const plainResponse = JSON.parse(JSON.stringify(response, bigIntReplacer));
    
    console.log('Response type:', typeof response);
    console.log('Response structure:', plainResponse);
    
    return { output: plainResponse };
  }
}
