cask "obsidian-nexus" do
  version "PLACEHOLDER_VERSION"
  sha256 "PLACEHOLDER_SHA256"

  url "https://github.com/gorillaKim/obsidian-nexus/releases/download/v#{version}/Obsidian-Nexus.dmg"
  name "Obsidian Nexus"
  desc "Agent-friendly knowledge search engine for Obsidian vaults"
  homepage "https://github.com/gorillaKim/obsidian-nexus"

  app "Obsidian Nexus.app"

  binary "#{appdir}/Obsidian Nexus.app/Contents/MacOS/nexus"
  binary "#{appdir}/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server"

  zap trash: [
    "~/.nexus",
    "~/Library/Application Support/com.obsidian-nexus.app",
    "~/Library/Caches/com.obsidian-nexus.app",
    "~/Library/Preferences/com.obsidian-nexus.app.plist",
    "~/Library/Logs/com.obsidian-nexus.app",
  ]
end
