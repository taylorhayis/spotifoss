#!/bin/bash

set -eo pipefail

REPO_OWNER="skyline69"
REPO_NAME="spotifoss"

cat <<EOF
cask "spotifoss" do
  version :latest
  sha256 :no_check

  url "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/latest/download/Spotifoss.dmg"
  name "Spotifoss"
  desc "Fast and native Spotify client"
  homepage "https://github.com/${REPO_OWNER}/${REPO_NAME}/"

  depends_on macos: ">= :big_sur"

  app "Spotifoss.app"

  zap trash: [
    "~/Library/Application Support/Spotifoss",
    "~/Library/Caches/com.spotifoss.app",
    "~/Library/Caches/Spotifoss",
    "~/Library/HTTPStorages/com.spotifoss.app",
    "~/Library/Preferences/com.spotifoss.app.plist",
    "~/Library/Saved Application State/com.spotifoss.app.savedState",
  ]
end
EOF
