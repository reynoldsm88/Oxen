# Oxen 🐂

Libraries and tools to manage Oxen repositories.

# Components

oxen, oxen-server, liboxen

## Commands

`oxen init .`

`oxen status`

`oxen add images/`

`oxen status`

`oxen commit -m "added images"`

`oxen push`


## File Structure

```
.oxen/
  HEAD (file that contains name of current "ref")

    ex) heads/main

  refs/ (keeps track of branch heads, remote names and their current commits)
    key,value db of:

    # Local heads
    heads/main -> COMMIT_ID
    heads/feature/add_cats -> COMMIT_ID
    heads/experiment/add_dogs -> COMMIT_ID

    # What has been pushed in these branches
    remotes/experiment/add_dogs -> COMMIT_ID

  staged/ (created from `oxen add <file>` command)
    key,value db of:

    filenames -> {"hash" => "FILE_HASH", "id" => "UUID_V4"} (we generate a file ID and hash for each file that is added)
    dirnames -> count

  commits/ (created from `oxen commit -m "my message"` command. Also generates history/commit_hash)
    key,value of:

    COMMIT_HASH -> Commit

    A Commit is an object that contains, can use parent for ordering the commit logs
      - Message
      - Parent Commit ID
      - Author
      - Timestamp

  history/ (list of commits)
    COMMIT_HASH_1/
      key,value of:

      filename -> { (filename is where we copy the version back to)
        "hash" => "FILE_HASH", (use this to know if a file was different)
        "is_synced" => false (used to know if it has been synced to server yet)
      }

    COMMIT_HASH_2/
    COMMIT_HASH_3/

  versions/ (copies of original files, versioned with commit ids)
    // TODO: make subdirs based on first two chars of hash, which would mean we have ~16^2=256 top level dirs, then 256 in /////       each, which would spread out the data nicely. If you take logbase 256 that means we can have a billion examples ///       split into the 4 levels easily
    //      (I think git does something somewhat similar?)
    // 
    //       ex) 59E029D4812AEBF0 -> 59/E0/29D4812AEBF0
    //           72617025710EBB55 -> 72/61/7025710EBB55
    //
    // TODO: use best lossless compression type based on file type, fall back to zlib or something for rest
    // TODO: maybe create watcher program to catch and intercept on write? Is this possible?
    FILE_UUID_1/
      COMMIT_ID_1 (dog_1.jpg)
      COMMIT_ID_2 (dog_1.jpg version 2)
    FILE_UUID_2/
      COMMIT_ID_1 (dog_2.jpg)
```

# Oxen Server

## Structure

Directories with repository names to simply sync data to

## APIs

set SERVER 0.0.0.0:3000

`curl "http://$SERVER/repositories"`

```
```
