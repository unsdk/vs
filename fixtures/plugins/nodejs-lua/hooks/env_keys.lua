function PLUGIN:EnvKeys(ctx)
  return {
    { key = "NODEJS_HOME", value = ctx.path },
    { key = "VS_NODEJS_HOME", value = ctx.path },
    { key = "PATH", value = ctx.path .. "/bin" },
  }
end
