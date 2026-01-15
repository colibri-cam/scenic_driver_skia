ExUnit.start()

Logger.configure(level: :debug)

{:ok, _apps} = Application.ensure_all_started(:scenic)

Code.require_file("support/view_port_helper.exs", __DIR__)
