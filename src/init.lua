lish = {
    status = 0
}


function lish.prompt()
    if lish.status == 0 then
        return pwd() .. "$ "
    else
        return pwd() .. "|" .. lish.status .. "$ "
    end
end
