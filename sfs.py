import os
import sys
import data
import tools
import readline

readline.parse_and_bind("tab: complete")

def loadSFS(storageDirectory):
    try:
        os.chdir(storageDirectory)
    except:
        print("Storage directory not found! Creating new directory...")
        os.mkdir(storageDirectory); os.chdir(storageDirectory)

    while True:
        currentDirectory = tools.getCurrentDir()
        try:
            promptText = data.prompt
            promptText = promptText.replace("!version!", data.programVersion)
            promptText = promptText.replace("!versionType!", data.versionType)
            promptText = promptText.replace("!updateTime!", data.updateTime)
            promptText = promptText.replace("!accessKey!", data.accessKey)
            promptText = promptText.replace("!commandCount!", str(tools.commandCount))
            promptText = promptText.replace("!directory!", currentDirectory)
            prompt = input("\n" + promptText)
        except KeyboardInterrupt:
            print("\n\nSFS.PROMPT.ERROR:KEYBOARD_INTERRUPT")
            sys.exit()
        
        print(tools.parse(prompt))

def setupSFS(storageDirectory):
    print("Welcome to SFS! Type 'help' to show a list of commands.")
    print("SFS was made as a virtual filesystem for servers, and it allows you to manage all your files safely.")
    print("To quit SFS, simply type 'quit' and all your data will be saved")

    try:
        tools.modifyData("firstRun", "0")
        os.mkdir(storageDirectory)
    except:
        pass

    loadSFS(storageDirectory)

def init(arguments):
    storageDirectory = "storage"
    if len(arguments) > 1:
        storageDirectory = arguments[1]
    if data.firstRun == 0:
        loadSFS(storageDirectory)
    else:
        setupSFS(storageDirectory)

