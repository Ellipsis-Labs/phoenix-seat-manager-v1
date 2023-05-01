const path = require('path');
const programDir = join(__dirname, '..');
const idlDir = join(__dirname, 'idl');
const sdkDir = join(__dirname, 'src', 'generated');
const binaryInstallDir = join(__dirname, '.crates');

export default {
    idlGenerator: 'shank',
    programName: 'phoenix_seat_manager',
    programId: 'PSMxQbAoDWDbvd9ezQJgARyq6R9L5kJAasaLDVcZwf1',
    idlDir,
    sdkDir,
    binaryInstallDir,
    programDir,
};