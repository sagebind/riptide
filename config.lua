function ls(...)
    return command("ls", "-l", "--color=always", ...)
end
