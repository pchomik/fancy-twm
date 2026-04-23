FancyTWM_Send(command, args := "") {
    static PipePath := "\\.\pipe\fancytwm-pipe"

    ; Build the JSON arguments list
    argString := ""
    if (IsObject(args) && args.Length > 0) {
        for index, value in args {
            ; Escape double quotes in values if they exist
            cleanValue := StrReplace(value, '"', '\"')
            argString .= '"' cleanValue '"' (index < args.Length ? ", " : "")
        }
    }

    payload := '{ "command": "' . command . '", "args": [' . argString . '] }'

    try {
        if !(pipe := FileOpen(PipePath, "w", "UTF-8"))
            throw Error("Pipe not found")

        pipe.Write(payload)
        pipe.Close()
        return true
    } catch {
        return false ; Silently fail so the user isn't interrupted
    }
}
