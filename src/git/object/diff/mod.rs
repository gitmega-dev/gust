//!
//! TODO:
//!  # 关于diff 的相关讨论
//! 1. 对于diff算法的选择  在myers与patience对比下明显myers更好
//! 对于 imara-diff 库， 因为包装归于
//! 
//! 每次的复制 字节大小为u8 *4 ,size 为u8*3
//! 

use std::vec;

use crate::utils;

use super::Metadata;
use super::types::ObjectType;

use diffs::Diff;
use diffs::myers;
#[allow(dead_code)]
#[derive(Debug)]
struct DeltaDiff{
   /// keep all instruction
   ops:Vec<DeltaOp>,
   old_data:Metadata,
   new_data:Metadata,
   ///Structural Similarity,相似性
   ssam:usize,
   ssam_r:f64,
 

}

impl DeltaDiff{
    pub fn new (old:Metadata,new:Metadata) -> Self{
        assert_eq!(old.t,new.t);
        let mut _new = DeltaDiff{
            ops:vec![],
            old_data:old.clone(),
            new_data:new.clone(),

            ssam: 0,
            ssam_r :0.00,
        };

        myers::diff(&mut _new, 
            &old.data, 0, old.data.len(), 
             &new.data, 0,  new.data.len()).unwrap();
        _new
    }

    pub fn get_delta_metadata(&mut self) -> Vec<u8>{
        
        let mut result:Vec<u8>=vec![];
        
        // 解码后长度编码
        //BUG : 更改这里的读取
        result.append(&mut utils::write_size_encoding(self.old_data.size )) ;
        result.append(&mut utils::write_size_encoding(self.new_data.size )) ;

        // 编码格式
        for op in &self.ops {
           result.append(&mut self.decode_OP(op)) ;
        }
        result

    }


    fn decode_OP(&self,op:&DeltaOp) -> Vec<u8>{
        let mut  op_data=vec![];
        match op.ins {
            Optype::DATA => {
                assert!(op.len<0x7f);
                let instruct =  (op.len & 0x7f) as u8 ;
                op_data.push(instruct);
                op_data.append(&mut self.new_data.data[op.begin..op.begin+op.len].to_vec());
            },
            Optype::COPY => {
                //TODO 暂时不考虑超出范围的情况
                let mut instruct:u8 = 0x80;
                let mut offset = op.begin;
                let mut size = op.len;
                let mut copy_data = vec![];
                assert!(op.len<0x1000000);
                for i in 0..4{
                    let _bit = (offset & 0xff) as u8;
                    if  _bit!= 0{
                        instruct |= (1<<i) as u8;
                        copy_data.push(_bit)
                    }
                    offset >>= 8;
                }
                for i in 4..7{
                    let _bit = (size & 0xff) as u8;
                    if  _bit!= 0{
                        instruct |= (1<<i) as u8;
                        copy_data.push(_bit)
                    }
                    size >>= 8;
                }
                op_data.push(instruct);
                op_data.append(&mut copy_data);

            },
        }
        op_data
    }

}



#[derive(Debug,Clone, Copy)]
enum Optype {
    DATA, // 插入的数据
    COPY, // 数据复制
}
#[allow(dead_code)]
#[derive(Debug,Clone, Copy)]
struct DeltaOp{
    /// instruction type
    ins:Optype, 
    /// data begin position
    begin: usize,
    /// data long 
    len:usize,

}

impl DeltaDiff {
    fn conver_to_delta(&self)-> Vec<u8>{
        todo!();
        // let mut result  =  Vec::new();
        // for op in &self.ops {
        //     todo!()
        // }
        // vec![];
    }
}
impl Diff for DeltaDiff{
    type Error = ();
    /// offset < 2^32
    /// len < 2^24
    fn equal(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), Self::Error> {
        //println!("equal {:?} {:?} {:?}", _old, _new, _len);
        self.ssam+=_len;
        self.ops.push(DeltaOp{ins:Optype::COPY,begin:_old,len:_len,});
        Ok(())
    }

    ///  insert  _len < 2 ^ 7
    fn insert(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), ()> {
        //println!("insert {:?} {:?} {:?}", _o, _n, _len);
        self.ops.push(DeltaOp{ins:Optype::DATA,begin:_new,len:_len,});
        Ok(())
    }


    fn finish(&mut self) -> Result<(), Self::Error> {
        self.ssam_r = self.ssam as f64 / self.new_data.data.len() as f64 ;
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use std::io::Write;

    use crate::{git::{object::{Metadata, types::ObjectType}, pack::{Pack, decode::ObjDecodedMap}}, utils};
    use bstr::ByteSlice;
    use diffs::myers;
    use super::DeltaDiff;

        #[test]
        fn test_metadata_diff_ofs_delta(){
            let m1 = Metadata::read_object_from_file
            ("./resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e".to_string()).unwrap();
            let mut m2 = Metadata::read_object_from_file
            ("./resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3".to_string()).unwrap();
            let mut diff = DeltaDiff::new(m1.clone(),m2.clone());
            println!("{:?}",diff);
            let meta_vec1 = m1.convert_to_vec().unwrap();


            //不需要压缩
            let offset_head = utils::write_offset_encoding(meta_vec1.len() as u64);
            // 166  - 12 = 154
            //不需要压缩
            let mut yasuo = diff.get_delta_metadata();
            m2.change_to_delta(ObjectType::OffsetDelta,yasuo,offset_head);

            
            let meta_vec = vec![m1,m2];
            let mut _pack = Pack::default();
            let pack_file_data =_pack.encode( Some(meta_vec));
            //_pack
            let mut file = std::fs::File::create("delta.pack").expect("create failed");
            file.write_all(pack_file_data.as_bytes()).expect("write failed");
            let a = Pack::decode_file("delta.pack");


        }


        #[test]
        fn test_metadata_diff_ref_delta(){
            let m1 = Metadata::read_object_from_file
            ("./resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e".to_string()).unwrap();
            let mut m2 = Metadata::read_object_from_file
            ("./resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3".to_string()).unwrap();
            let mut diff = DeltaDiff::new(m1.clone(),m2.clone());
            println!("{:?}",diff);
            let meta_vec1 = m1.convert_to_vec().unwrap();
            

            //不需要压缩
            let offset_head =m1.id.0.to_vec();
            assert!(offset_head.len() ==20);
            // 166  - 12 = 154
            //不需要压缩
            let mut yasuo = diff.get_delta_metadata();
            m2.change_to_delta(ObjectType::HashDelta,yasuo,offset_head);

            
            let meta_vec = vec![m1,m2];
            let mut _pack = Pack::default();
            let pack_file_data =_pack.encode( Some(meta_vec));
            //_pack
            let mut file = std::fs::File::create("delta.pack").expect("create failed");
            file.write_all(pack_file_data.as_bytes()).expect("write failed");
            let a = Pack::decode_file("delta.pack");

            let mut result = ObjDecodedMap::default();
            result.update_from_cache(&a.get_cache());
            
            for (key, value) in result._map_hash.iter() {
                println!("*********************");
                println!("Hash :{}", key);
                println!("{}", value);
            }
        }


}