export type ZBBError = NotInANetwork | NotInSameNetwork | ADBError | IO | Other;

type NotInANetwork = {
    type: 'NotInANetwork'
}

type NotInSameNetwork = {
    type: 'NotInSameNetwork'
}

type ADBError = {
    type: 'ADB',
    message: string
}

type IO = {
    type: 'IO',
    message: string
}

type Other = {
    type: 'Other',
    message: string
}