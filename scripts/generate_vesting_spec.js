const { decodeAddress, encodeAddress } = require('@polkadot/util-crypto');
const { u8aToHex, isHex, hexToU8a } = require('@polkadot/util');

const fs = require('fs');
const csv = require('csv-parser');

const isValidSubstrateAddress = (address) => {
  try {
    encodeAddress(isHex(address) ? hexToU8a(address) : decodeAddress(address));

    return true;
  } catch (error) {
    return false;
  }
};

async function main() {
  const inputFilePath = process.argv[2];
  const blockStartDate = process.argv[3] || '16/10/2021';
  const blockDuration = 12; // seconds
  const investors = [];
  const dates = [
    '16/10/2021',
    '16/01/2022',
    '16/04/2022',
    '16/07/2022',
    '16/10/2022',
    '16/01/2023',
    '16/04/2023',
    '16/07/2023',
    '17/10/2023'
  ];
  // const vestingtime = 365.25 * 24 / 4 * 60 * 60 / blockDuration;  // 3 months
  const vestingtime = (3600*24) / blockDuration;  // Blocks / hour

  const PdexUnit = "000000000";

  const getBlockNumber = (str) => {
    const [d0, m0, y0] = blockStartDate.split('/');
    const [d1, m1, y1] = str.split('/');
    const t0 = new Date(y0, m0, d0, 0, 0, 0, 0);
    const t1 = new Date(y1, m1, d1, 0, 0, 0, 0);
    return 100+(t1.getTime() - t0.getTime()) / blockDuration /1000;
  };

  fs.createReadStream(inputFilePath)
    .pipe(csv())
    .on('data', function (row) {
      try {
        if (!['Address', 'N/A', '-', ''].includes(row.Address)) {
          const publicKey = decodeAddress(row.Address);
          const hexPublicKey = u8aToHex(publicKey);

          if (!isValidSubstrateAddress(hexPublicKey)) {
            throw `Invalid Substrate address: ${row.Address} for ${row.Investors}`;
          }

          let total = 0;
          const amounts = dates.map((d) => {
            const amount =Math.trunc(1000 * (row[d].length ? parseFloat(row[d]) : 0 ));
            total += amount;
            return amount;
          });
          let comment = ""; // "        /* "+row.Address+"  -  "+row.Investors+" */\n";  // TODO check for -d flag
          investors.push({
            address: hexPublicKey.substr(2),
            amounts,
            total,
            comment,
          });
        }
      } catch (err) {
        console.error("/* TODO "+err+"*/");
        // process.exit();
      }
    })
    .on('end', function () {
      let balances = `    let mut investor_balances = vec![
`;
      let vesting = `    let investor_vesting = vec![
`;
      investors.map((inv) => {
         // TODO Technical dept this will add 0.2 tokens as free to all acount
         // to be able to pay for the fee to release the tokens.
        balances += `${inv.comment}       (hex!["${inv.address}"].into(), ${inv.total+50}${PdexUnit}),
`;
        let value = inv.amounts[0];
        let stIndex = 0;
        for (let i = 1; i < inv.amounts.length; i++) {
          if (value !== inv.amounts[i]) {
            vesting += `${inv.comment}       (hex!["${inv.address}"].into(), ${getBlockNumber(dates[stIndex])}, ${vestingtime}, ${i - stIndex}, ${value}${PdexUnit}),
`;
            stIndex = i;
            value = inv.amounts[i];
          }
          // else{
          //   console.log("Problem"+inv.Investors);
          // }
        }
      });

      balances += `    ];`;
      vesting += `    ];`;

      console.log(balances);
      console.log(vesting);
    });
}

main().catch((error) => {
  console.error(error);
  process.exit(-1);
});
