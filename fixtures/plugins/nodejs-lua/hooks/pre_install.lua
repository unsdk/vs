function PLUGIN:PreInstall(ctx)
  if ctx.version == "20.11.1" then
    return {
      version = "20.11.1",
      url = "packages/20.11.1",
    }
  end

  if ctx.version == "18.19.0" then
    return {
      version = "18.19.0",
      url = "packages/18.19.0",
    }
  end

  return nil, "version not found"
end
