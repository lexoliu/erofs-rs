use alloc::vec::Vec;
use typed_path::{UnixPath, UnixPathBuf};

use super::EroFS;
use crate::backend::AsyncImage;
use crate::dirent::{DirEntry, DirPosition, DirentBlock};
use crate::error::Widen;
use crate::{Result, types::Inode};

/// The entries of one directory, in on-disk order, without reading their inodes.
///
/// Created by [`EroFS::read_dir_inode`] or [`EroFS::read_dir_inode_at`].
pub struct ReadDir<'a, I: AsyncImage> {
    dir: UnixPathBuf,
    pub(super) inode: Inode,
    erofs: &'a EroFS<I>,
    dirent_block: DirentBlock<Vec<u8>>,
    offset: u64,
}

impl<'a, I: AsyncImage> ReadDir<'a, I> {
    pub(crate) async fn new<P: AsRef<UnixPath>>(
        erofs: &'a EroFS<I>,
        inode: Inode,
        dir: P,
    ) -> Result<Self, I::Error> {
        Self::at(erofs, inode, dir, DirPosition::START).await
    }

    /// Opens the listing at `position`, which must fall on an entry of the
    /// directory or be its end.
    pub(crate) async fn at<P: AsRef<UnixPath>>(
        erofs: &'a EroFS<I>,
        inode: Inode,
        dir: P,
        position: DirPosition,
    ) -> Result<Self, I::Error> {
        let size = inode.data_size();
        // At the end of the listing the first block stands in, never read from.
        let end = position.raw() >= size;
        let (block_offset, index) = if end {
            (0, 0)
        } else {
            position.split(erofs.block_size())
        };
        let block_data = erofs.read_inode_block(&inode, block_offset).await?;
        let mut dirent_block = DirentBlock::new(block_data).widen()?;
        dirent_block.seek(index).widen()?;
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

    pub async fn next_entry(&mut self) -> Result<Option<DirEntry>, I::Error> {
        while self.offset < self.inode.data_size() {
            if let Some(entry) = self.dirent_block.next_entry(&self.dir).widen()? {
                return Ok(Some(entry));
            }
            let offset = self.offset + self.dirent_block.block_size() as u64;
            if offset < self.inode.data_size() {
                let block = self.erofs.read_inode_block(&self.inode, offset).await?;
                self.dirent_block = DirentBlock::new(block).widen()?;
            }
            self.offset = offset;
        }
        Ok(None)
    }
}
