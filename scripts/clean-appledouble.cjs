const fs = require('node:fs/promises');
const path = require('node:path');

async function removeAppleDouble(directory) {
  let entries;
  try {
    entries = await fs.readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (error.code === 'ENOENT') return;
    throw error;
  }

  await Promise.all(entries.map(async (entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.name.startsWith('._')) {
      await fs.rm(entryPath, { recursive: true, force: true });
      return;
    }
    if (entry.isDirectory()) {
      await removeAppleDouble(entryPath);
    }
  }));
}

exports.default = async function cleanAppleDouble(context) {
  await removeAppleDouble(context.appOutDir);
};
