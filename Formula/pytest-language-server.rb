class PytestLanguageServer < Formula
  desc "Blazingly fast Language Server Protocol implementation for pytest"
  homepage "https://github.com/bellini666/pytest-language-server"
  version "0.17.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.17.0/pytest-language-server-aarch64-apple-darwin"
      sha256 "2828eef961bbebc3215d5ac4d53df957d3fcbad2d33982981c92959249d7d1a2"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.17.0/pytest-language-server-x86_64-apple-darwin"
      sha256 "8a7269ae5cd5003ba6145def3dbb81f4ac47dd4b1e59049e9f6a94db1cfd50d1"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.17.0/pytest-language-server-aarch64-unknown-linux-gnu"
      sha256 "6190b1a27ad4dcd929a2c90daaf8f44e475a8c7477ed2580e541df2ad1ec17c9"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.17.0/pytest-language-server-x86_64-unknown-linux-gnu"
      sha256 "e460afd92c060ea9dff0ede85c9233ae032f799d3a81f333fc1e286e39b2b2b4"
    end
  end

  def install
    bin.install cached_download => "pytest-language-server"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pytest-language-server --version")
  end
end
