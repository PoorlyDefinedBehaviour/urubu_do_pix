## Transcoding streaming with ffmpeg

### Works

ffmpeg -i http://127.0.0.1:11470/9d6bc3eab9687dcfe75b2933e7b46872726580aa/1 -listen 1 -preset fast -f mp4 -crf 20 -movflags frag_keyframe+empty_moov http://localhost:3001/video_stream

ffmpeg -i http://127.0.0.1:11470/8307ab4fa93406542add098a840c8d6c3db5369b/2 -listen 1 -preset fast -f mp4 -crf 20 -movflags frag_keyframe+empty_moov http://localhost:3001/video_stream

# Running the bot

Enigo(to press keyboard keys) has dependencies:

```
sudo apt install libxdo-dev
```

```
docker-compose -f docker-compose-dev.yaml up -d

# Create a symlink so selenium can find google chrome.
sudo ln -s /usr/bin/google-chrome /usr/bin/chrome

# Selenium must be running in order to stream video on discord because we need to interact with a browser.
java -Dwebdriver.chrome.driver=/usr/bin/chromedriver -jar selenium-server-4.5.0.jar standalone

cargo run
```

## Installing selenium + chromedriver

```
sudo su
sudo curl -sS -o - https://dl-ssl.google.com/linux/linux_signing_key.pub | apt-key add
sudo bash -c "echo 'deb [arch=amd64] http://dl.google.com/linux/chrome/deb/ stable main' >> /etc/apt/sources.list.d/google-chrome.list"
sudo apt -y update
sudo apt -y install google-chrome-stable
sudo apt update
sudo apt install -y unzip xvfb libxi6 libgconf-2-4
wget https://chromedriver.storage.googleapis.com/106.0.5249.61/chromedriver_linux64.zip
unzip chromedriver_linux64.zip
sudo mv chromedriver /usr/bin/chromedriver
sudo chown root:root /usr/bin/chromedriver
sudo chmod +x /usr/bin/chromedriver
```

https://tecadmin.net/setup-selenium-chromedriver-on-ubuntu/

## TODO

When you mention someone, the conversation bot uses the discord id instead of the user name, fix it.

Classify things people talk about to find trends
