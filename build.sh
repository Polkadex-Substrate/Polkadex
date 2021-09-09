sudo apt install -y git clang curl libssl-dev llvm libudev-dev
curl https://getsubstrate.io -sSf | bash -s -- --fast
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
cargo build --release



cc1d2a1775ef5d018720807b36e35d75d47b2ea53d3164e37535eac972bb3f62 >> 12D3KooWPQuiRGetkirVUgJ5cmS3HPNNbjpGwSGsZDwocEKRvhuU

cd $HOME
curl -O -L https://github.com/Polkadex-Substrate/Polkadex/releases/download/v0.4.1-rc3/customSpecRaw.json
$HOME/Polkadex/target/release/polkadex-node --chain=$HOME/udonGenesisRaw.json --bootnodes /ip4/13.235.190.203/tcp/30333/p2p/12D3KooWPQuiRGetkirVUgJ5cmS3HPNNbjpGwSGsZDwocEKRvhuU --validator --name 'Polkadex Team'

$HOME/Polkadex/target/release/polkadex-node --chain=$HOME/customSpecRaw.json  --validator --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' --name 'Polkadex Team' --node-key cc1d2a1775ef5d018720807b36e35d75d47b2ea53d3164e37535eac972bb3f62




server {

        server_name blockchain.polkadex.trade;

        root /var/www/html;
        index index.html;

        location / {
          try_files $uri $uri/ =404;

          proxy_buffering off;
          proxy_pass http://localhost:9944;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header Host $host;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

          proxy_http_version 1.1;
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection "upgrade";
        }

        listen [::]:443 ssl ipv6only=on;
        listen 443 ssl;
        ssl_certificate /home/ubuntu/local/letsencrypt/live/blockchain.polkadex.trade/fullchain.pem;
        ssl_certificate_key /home/ubuntu/local/letsencrypt/live/blockchain.polkadex.trade/privkey.pem;

        ssl_session_cache shared:cache_nginx_SSL:1m;
        ssl_session_timeout 1440m;

        ssl_protocols TLSv1 TLSv1.1 TLSv1.2;
        ssl_prefer_server_ciphers on;

        ssl_ciphers "ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-SHA256:ECDHE-RSA-AES128-SHA256:ECDHE-ECDSA-AES128-SHA:ECDHE-RSA-AES256-SHA384:ECDHE-RSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA384:ECDHE-ECDSA-AES256-SHA:ECDHE-RSA-AES256-SHA:DHE-RSA-AES128-SHA256:DHE-RSA-AES128-SHA:DHE-RSA-AES256-SHA256:DHE-RSA-AES256-SHA:ECDHE-ECDSA-DES-CBC3-SHA:ECDHE-RSA-DES-CBC3-SHA:EDH-RSA-DES-CBC3-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA256:AES256-SHA256:AES128-SHA:AES256-SHA:DES-CBC3-SHA:!DSS";

        ssl_dhparam /etc/ssl/certs/dhparam.pem;

}