class PytestLanguageServer < Formula
  desc "Blazingly fast Language Server Protocol implementation for pytest"
  homepage "https://github.com/bellini666/pytest-language-server"
  version "0.19.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.1/pytest-language-server-aarch64-apple-darwin"
      sha256 "958b4d42d23f4f8198742e35cc3dfd861bf6735a1fc9a0ae44971c2614f007d0"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.1/pytest-language-server-x86_64-apple-darwin"
      sha256 "1df377d1551e21e4b675a41535622eb5d1957856ef1044c1feecb6f13647a820"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.1/pytest-language-server-aarch64-unknown-linux-gnu"
      sha256 "9f536a8e14e8e039312c886dee8509017ce1a5d3e6e88d016179790df3a711eb"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.1/pytest-language-server-x86_64-unknown-linux-gnu"
      sha256 "fcc6be37cd77022d263f69a68f5e7b78226dbbb8d32d5b0c1bb56f009171b28a"
    end
  end

  def install
    bin.install cached_download => "pytest-language-server"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pytest-language-server --version")
  end
end
