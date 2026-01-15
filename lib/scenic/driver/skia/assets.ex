defmodule Scenic.Driver.Skia.Assets do
  use Scenic.Assets.Static,
    otp_app: :scenic_driver_skia,
    sources: ["assets", {:scenic, "assets"}],
    aliases: [
      roboto: "fonts/roboto.ttf",
      roboto_mono: "fonts/roboto_mono.ttf",
      test_red: "images/test_red.png",
      stock: "images/stock.jpg"
    ]
end
