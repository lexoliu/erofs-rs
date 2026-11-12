use alloc::borrow::Cow;
use typed_path::{UnixPath, UnixPathBuf};

use super::EroFS;
use crate::backend::Image;
use crate::dirent::{DirEntry, DirPosition, DirentBlock};
use crate::{Result, types::Inode};

/// The entries of one directory, in on-disk order, without reading their inodes.
///
/// Created by [`EroFS::read_dir_inode`] or [`EroFS::read_dir_inode_at`].
#[derive(Debug)]
pub struct ReadDir<'a, I: Image> {
    dir: UnixPathBuf,
    pub(super) inode: Inode,
    erofs: &'a EroFS<I>,
    dirent_block: DirentBlock<Cow<'a, [u8]>>,
    offset: u64,
}

impl<'a, I: Image> ReadDir<'a, I> {
    pub(crate) fn new<P: AsRef<UnixPath>>(
        erofs: &'a EroFS<I>,
        inode: Inode,
        dir: P,
    ) -> Result<Self> {
        Self::at(erofs, inode, dir, DirPosition::START)
    }

    /// Opens the listing at `position`, which must fall on an entry of the
    /// directory or be its end.
    pub(crate) fn at<P: AsRef<UnixPath>>(
        erofs: &'a EroFS<I>,
        inode: Inode,
        dir: P,
        position: DirPosition,
    ) -> Result<Self> {
        let size = inode.data_size();
        // At the end of the listing the first block stands in, never read from.
        let end = position.raw() >= size;
        let (block_offset, index) = if end {
            (0, 0)
        } else {
            position.split(erofs.block_size())
        };
        let block = erofs.get_inode_block(&inode, block_offset)?;
        let mut dirent_block = DirentBlock::new(block)?;
        dirent_block.seek(index)?;
        Ok(Self {
            dir: dir.as_ref().to_path_buf(),
            inode,
            erofs,
            dirent_block,
            offset: if end { size } else { block_offset },
        })
    }

    /// Where the listing resumes after the entries it has yielded.
    pub fn position(&self) -> DirPosition {
        if self.offset >= self.inode.data_size() {
            DirPosition::new(self.inode.data_size())
        } else {
            DirPosition::at(self.offset, self.dirent_block.next_index())
        }
    }

    fn next_entry(&mut self) -> Result<Option<DirEntry>> {
        while self.offset < self.inode.data_size() {
            if let Some(entry) = self.dirent_block.next_entry(&self.dir)? {
                return Ok(Some(entry));
            }
            let offset = self.offset + self.dirent_block.block_size() as u64;
            if offset < self.inode.data_size() {
                let block = self.erofs.get_inode_block(&self.inode, offset)?;
                self.dirent_block = DirentBlock::new(block)?;
            }
            self.offset = offset;
        }
        Ok(None)
    }
}

impl<'a, I: Image> Iterator for ReadDir<'a, I> {
    type Item = Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_entry().transpose()
    }
}
