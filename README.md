# Notion Export Cleaner

## Why ?

When exporting a [Notion](https://www.notion.so/) workspace, the resulting files come all with an additional suffix that uniquely identifies the page.
This is surely an internal ID that allows to rename pages without breaking links.
This may be useful for exports too, if you want to track changes between exports.
However, in a common use case of an export - to backup your pages - this is not that useful and makes the files harder to read.

This tools aims to rename the files to their original name, and remove the suffix.
It will add a suffix only if there are multiple pages with the same name.
It is not trivial, since you also need to edit all links everywhere in the files.

## How to use it ?

1. **Export :**
   You'll need to export your Notion workspace as markdown or html files, with subpages as folders.
![export settings](./doc/export_settings.png)
> NB: you can also export as markdown or html _without_ subpages as folders. However, there will be a lot more name conflicts (multiple pages wanting to be called the same), so a lot more `My file`, `My file 1`, `My file 2`, etc.

1. **Unzip :**
   Once the export is done, you'll have a zip file. This zip file will be difficult to unpack. In a lot of cases, Windows will refuse to unzip it, due to the long file names. My personal recommandation for unziping is to do the following:

   1. Use `7-zip` to "extract here" the zip file. This will create a folder with the same name as the zip file, but wont extract it recursively.
   
   2. Rename the zip to something shorter, like `export.zip`
   
   3. Use `7-zip` to "extract files". Choose a destination with a short path. I personnaly use `D:\`, the root of a secondary drive.

2. **Run the executable :**
   Inside a terminal, run the executable with the path to the folder containing the exported files as argument:
   `/path/to/notion-export-cleaner.exe /path/to/exported/folder`

## Additional options

- `-h` or `--help` : display the help
- `-V` or `--version` : display the version
- `-i` or `--ignore` : list of file or directory names to ignore, separated by commas. For example: `-i .git,index.html` will ignore the `.git` folder and the `index.html` file. This means that no renaming and no link editing will be done on these files.

Example: `/path/to/notion-export-cleaner.exe /path/to/exported/folder -i .git`

## Example of output:

| Before | After  |
| ------ | ------ |
| ![before](./doc/file_tree_before.png) | ![after](./doc/file_tree_after.png) |