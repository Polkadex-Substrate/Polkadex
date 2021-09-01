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
  const blockStartDate = process.argv[3] || '15/09/2021';
  const blockDuration = 3000;
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

  const getBlockNumber = (str) => {
    const [d0, m0, y0] = blockStartDate.split('/');
    const [d1, m1, y1] = str.split('/');
    const t0 = new Date(y0, m0, d0, 0, 0, 0, 0);
    const t1 = new Date(y1, m1, d1, 0, 0, 0, 0);
    return (t1.getTime() - t0.getTime()) / blockDuration;
  };

  fs.createReadStream(inputFilePath)
    .pipe(csv())
    .on('data', function (row) {
      try {
        if (!['Address', 'N/A', '-', ''].includes(row.Address)) {
          const publicKey = decodeAddress(row.Address);
          const hexPublicKey = u8aToHex(publicKey);

          if (!isValidSubstrateAddress(hexPublicKey)) {
            throw `Invalid Substrate address: ${row.Address}`;
          }

          let total = 0;
          const amounts = dates.map((d) => {
            const amount = row[d].length ? parseFloat(row[d]) : 0;
            total += amount;
            return amount;
          });
          investors.push({
            address: hexPublicKey,
            amounts,
            total
          });
        }
      } catch (err) {
        console.error(err);
        process.exit();
      }
    })
    .on('end', function () {
      let balances = `balances: vec![
`;
      let vesting = `vesting: vec![
`;
      investors.map((inv) => {
        balances += `   (hex!["${inv.address}"].into(), ${inv.total}),
`;
        let value = inv.amounts[0];
        let stIndex = 0;
        for (let i = 1; i < inv.amounts.length; i++) {
          if (value !== inv.amounts[i]) {
            vesting += `    (hex!["${inv.address}"].into(), ${getBlockNumber(dates[stIndex])}, ${
              (30 * 24 * 60 * 60) / blockDuration
            }, ${i - stIndex}, ${value}),
`;
            stIndex = i;
            value = inv.amounts[i];
          }
        }
      });

      balances += `]`;
      vesting += `]`;

      console.log(balances);
      console.log(vesting);
    });
}

main().catch((error) => {
  console.error(error);
  process.exit(-1);
});
