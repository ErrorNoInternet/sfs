import os
import sys
import data
import glob
import codecs
import shutil
import pathlib
from cryptography.fernet import Fernet

commandCount = 0
fernet = Fernet(data.accessKey)
startDirectory = os.curdir

def parse(command):
    global commandCount
    commandCount += 1
    if command == "" or command == " ":
        return "SFS.PARSE.ERROR:EMPTY_CMD"
    elif command == "exit" or command == "quit":
        sys.exit()
    elif command == "help":
        for line in data.help:
            print(line)
        return "Displayed HELP Page"
    elif command == "ls" or command == "dir":
        directoryItems = os.listdir()
        if "sfs.lock" in directoryItems:
            directoryItems.remove("sfs.lock")
            directoryItems.append("SFS.LOCKED")
        directoryItems = str(directoryItems)
        directoryItems = directoryItems.replace("[", "")
        directoryItems = directoryItems.replace("]", "")
        directoryItems = directoryItems.replace(",", " ")
        return directoryItems
    elif "cd" in command:
        try:
            return changeDir(command.split("cd ")[1])
        except Exception as error:
            return "SFS.PARSE.ERROR:CD_INVALID_ARGUMENT." + str(error)
    elif "mkdir" in command:
        try:
            return makeDir(command.split("mkdir ")[1])
        except:
            return "SFS.PARSE.ERROR:MKDIR_INVALID_ARGUMENT"
    elif "rmdir" in command:
        try:
            return removeDir(command.split("rmdir ")[1])
        except:
            return "SFS.PARSE.ERROR:RMDIR_INVALID_ARGUMENT"
    elif "rndir" in command:
        try:
            return renameDir(command.split(" ")[1], command.split(" ")[2])
        except:
            return "SFS.PARSE.ERROR:RNDIR_INVALID_ARGUMENT"
    elif "rmfile" in command:
        try:
            return removeFile(command.split("rmfile ")[1])
        except:
            return "SFS.PARSE.ERROR:RMFILE_INVALID_ARGUMENT"
    elif "mkfile" in command:
        try:
            return makeFile(command.split("mkfile ")[1])
        except:
            return "SFS.PARSE.ERROR:MKFILE_INVALID_ARGUMENT"
    elif "encrypt" in command:
        try:
            return encryptFiles()
        except Exception as error:
            return "SFS.PARSE.ERROR:ENCRYPT_ERROR." + str(error)
    elif "decrypt" in command:
        try:
            return decryptFiles()
        except Exception as error:
            return "SFS.PARSE.ERROR:DECRYPT_ERROR." + str(error)
    elif "generatekey" in command:
        try:
            return generateKey()
        except:
            return "SFS.PARSE.ERROR:KEY_FAILURE"
    elif "changekey" in command:
        try:
            newKey = input("Key: ")
            return changeKey(newKey)
        except:
            return "SFS.PARSE.ERROR:CHANGE_KEY_FAILURE"
    elif "getkey" in command:
        try:
            return getKey()
        except:
            return "SFS.PARSE.ERROR:GET_KEY_FAILURE"
    elif "cat" in command:
        try:
            fileName = command.split("cat ")[1]
            return cat(fileName)
        except:
            return "SFS.PARSE.ERROR:CAT_FILE_FAILURE"
    elif "commandcount" in command or "cmdcount" in command:
        try:
            return getCommandCount()
        except:
            return "SFS.PARSE.ERROR:CAT_FILE_FAILURE"
    elif "python -u" in command and "main.py" in command:
        sys.exit()
    elif "os:" in command:
        try:
            return execute(command.split("os:")[1])
        except:
            return "SFS.PARSE.ERROR:SYS_CMD_INVALID_ARGUMENT"
    else:
        return "SFS.PARSE.ERROR:UNKNOWN_CMD"

def getCommandCount():
    return "Commands used: " + str(commandCount)

def cat(filename):
    file = open(filename, "r")
    fileData = file.read().splitlines()
    file.close()
    for line in fileData:
        print(line)
    return "Successfully displayed file contents"

def changeKey(newKey):
    print("\nCurrent Key: " + data.accessKey)
    modifyData("accessKey", f"'{newKey}'")
    print("New Key: " + newKey)
    return "Successfully changed key. Please restart SFS to use your new key."

def getKey():
    return "Current Key: " + data.accessKey

def generateKey():
    key = Fernet.generate_key()
    key = str(key).replace("b'", "'")
    return "Generated New Key: " + key

def modifyData(dataName, dataValue):
    dataIndex = 0
    if dataName == "accessKey":
        dataFile = open("../data.py", "r")
    else:
        dataFile = open("data.py", "r")
    dataValues = dataFile.readlines()
    dataFile.close()
    for name in dataValues:
        if dataName in name:
            dataValues[dataIndex] = "{} = {}\n".format(dataName, dataValue)
            break
        dataIndex += 1
    if dataName == "accessKey":
        dataFile = open("../data.py", "w")
    else:
        dataFile = open("data.py", "w")
    for line in dataValues:
        dataFile.write(line)
    dataFile.close()

def decryptFiles():
    allFiles = []
    try:
        os.remove("sfs.lock")
    except:
        pass
    for x in os.walk("."):
        for y in glob.glob(os.path.join(x[0], '*')):
            if os.path.isfile(y):
                allFiles.append(y)
    for filename in allFiles:
        try:
            try:
                file = open(filename, "rb")
                fileData = file.read()
            except:
                continue
            file.close()
            decrypted = fernet.decrypt(fileData)
            fileFolder = filename.split("/")[:-1]
            singleFileName = filename.split("/"); folderPath = ""; index = 0
            for folder in fileFolder:
                folderPath += folder + "/"
            singleFileName = singleFileName[len(singleFileName)-1]
            file = open(folderPath + fernet.decrypt(singleFileName.encode("utf-8")).decode("utf-8"), "wb+")
            file.write(decrypted); file.close(); os.remove(filename)
        except:
            pass
    return "Decrypted all files"

def encryptFiles():
    allFiles = []
    sfsLock = open("sfs.lock", "w")
    sfsLock.write("This folder is encrypted by SFS\nSecureFileSystem " + data.programVersion + " made by Ryan Huang")
    sfsLock.close()
    for x in os.walk("."):
        for y in glob.glob(os.path.join(x[0], '*')):
            if os.path.isfile(y):
                allFiles.append(y)
    allFiles.remove("./sfs.lock")
    for filename in allFiles:
        try:
            file = open(filename, "rb")
            fileData = file.read()
        except:
            continue
        file.close()
        encrypted = fernet.encrypt(fileData)
        fileFolder = filename.split("/")[:-1]
        singleFileName = filename.split("/"); folderPath = ""; index = 0
        for folder in fileFolder:
            folderPath += folder + "/"
        singleFileName = singleFileName[len(singleFileName)-1]
        file = open(folderPath + fernet.encrypt(singleFileName.encode("utf-8")).decode("utf-8"), "wb+")
        file.write(encrypted); file.close(); os.remove(filename)
    return "Encrypted all files"

def getCurrentDir():
    directory =  pathlib.Path().absolute()
    directory = str(directory)
    try:
        directory = directory.split("/storage")[1]
    except:
        directory = directory
    return directory

def changeDir(directory):
    os.chdir(directory)
    return "New Directory: " + directory

def makeDir(directory):
    os.mkdir(directory)
    return "Created Directory: " + directory

def removeDir(directory):
    shutil.rmtree(directory)
    return "Removed Directory: " + directory

def renameDir(directory, newName):
    os.rename(directory, newName)
    return "Renamed Directory {} to {}".format(directory, newName)

def removeFile(file):
    os.remove(file)
    return "Removed File: " + file

def execute(command):
    return os.system(command)

def makeFile(file):
    newFile = open(file, "w+")
    newFile.close()
    return "Created File: " + file

