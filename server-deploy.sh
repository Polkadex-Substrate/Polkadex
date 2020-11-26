sudo apt update
sudo apt install snapd
sudo snap install core
sudo snap refresh core
sudo snap install --classic certbot
sudo ln -s /snap/bin/certbot /usr/bin/certbot
sudo apt-get install python3-pip -y
pip3 install certbot-dns-route53
certbot plugins
export AWS_ACCESS_KEY_ID=<INSERT_AWS_ACCESS_KEY_ID>
export AWS_SECRET_ACCESS_KEY=<INSERT_AWS_SECRET_ACCESS_KEY>
certbot certonly -n --agree-tos --email business@polkadex.trade -d cloud.polkadex.trade --dns-route53 --preferred-challenges=dns --logs-dir /tmp/letsencrypt --config-dir ~/local/letsencrypt --work-dir /tmp/letsencrypt


echo 'server {

        server_name cloud.polkadex.trade;

        root /var/www/html;
        index index.html;

        location / {
          try_files $uri $uri/ =404;

          proxy_pass http://localhost:3000;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header Host $host;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

          proxy_http_version 1.1;
          proxy_set_header Upgrade $http_upgrade;
          proxy_set_header Connection "upgrade";
        }

        listen [::]:443 ssl ipv6only=on;
        listen 443 ssl;
        ssl_certificate /home/ubuntu/local/letsencrypt/live/cloud.polkadex.trade/fullchain.pem;
        ssl_certificate_key /home/ubuntu/local/letsencrypt/live/cloud.polkadex.trade/privkey.pem;

        ssl_session_cache shared:cache_nginx_SSL:1m;
        ssl_session_timeout 1440m;

        ssl_protocols TLSv1 TLSv1.1 TLSv1.2;
        ssl_prefer_server_ciphers on;

        ssl_ciphers "ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-SHA256:ECDHE-RSA-AES128-SHA256:ECDHE-ECDSA-AES128-SHA:ECDHE-RSA-AES256-SHA384:ECDHE-RSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA384:ECDHE-ECDSA-AES256-SHA:ECDHE-RSA-AES256-SHA:DHE-RSA-AES128-SHA256:DHE-RSA-AES128-SHA:DHE-RSA-AES256-SHA256:DHE-RSA-AES256-SHA:ECDHE-ECDSA-DES-CBC3-SHA:ECDHE-RSA-DES-CBC3-SHA:EDH-RSA-DES-CBC3-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA256:AES256-SHA256:AES128-SHA:AES256-SHA:DES-CBC3-SHA:!DSS";

        ssl_dhparam /etc/ssl/certs/dhparam.pem;

}' >> default
sudo apt-get install nginx
sudo mv default /etc/nginx/sites-enabled/
sudo openssl dhparam -out /etc/ssl/certs/dhparam.pem 2048
sudo service nginx restart
sudo service nginx status