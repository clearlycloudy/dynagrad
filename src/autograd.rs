use std::collections::HashMap;
use std::fmt;
use std::cell::Cell;
use std::cmp;

///implementation of reverse automatic differentiation
pub struct Context {
    _id: Cell< usize >,
    _id_map: HashMap< usize, usize >,
    _eval_order: Vec< usize >,
    _is_evaluated: usize,
    _buf: Vec< Link >,
    _eval_order_map: HashMap< usize, usize >,
}

#[derive(Clone, Debug)]
pub enum OpType {
    Concat,
    Mul,
    Div,
    AddParallel,
    AddAll,
    Sin,
    Cos,
    Tan,
    Exponential,
    Log,
    Sigmoid,
    // Softmax,
    Tanh,
    Relu,
    ReluLeaky,
}

impl Default for Context {
    fn default() -> Context {
        Context {
            _id: Cell::new( 0usize ),
            _id_map: HashMap::new(),
            _eval_order: vec![],
            _is_evaluated: <usize>::max_value(),
            _buf: vec![],
            _eval_order_map: HashMap::new(),
        }
    }
}

impl Context {
    fn gen_id( & mut self ) -> usize {
        let a = self._id.get();
        *self._id.get_mut() = a + 1usize;
        a + 1
    }
    pub fn get_var( & mut self, id: usize ) -> Option< & Link > {
        match self._id_map.get( &id ) {
            Some( &i ) => Some( & self._buf[i] ),
            _ => None,
        }
    }
    pub fn init( & mut self ) -> Link {
        let a : usize = self.gen_id();
        let mut l : Link = Default::default();
        l._id = a;
        l
    }
    pub fn init_var( & mut self, v: & [ f64 ] ) -> Link {
        let a : usize = self.gen_id();
        let mut l : Link = Default::default();
        l._id = a;
        l._val = v.to_vec();
        l
    }
    pub fn init_op( & mut self, op: OpType, args: & mut [ & mut Link ] ) -> Link {
        let a : usize = self.gen_id();
        let mut l : Link = Default::default();
        l._id = a;
        let b : Box< Op > = match op {
            OpType::Concat => { Box::new( OpConcat{ _arity: args.len() } ) },
            OpType::Mul => { Box::new( OpMul{} ) },
            OpType::Div => { Box::new( OpDiv{} ) },
            OpType::AddParallel => { Box::new( OpAddParallel{} ) },
            OpType::AddAll => { Box::new( OpAddAll{} ) },
            OpType::Sin => { Box::new( OpSin{} ) },
            OpType::Cos => { Box::new( OpCos{} ) },
            OpType::Tan => { Box::new( OpTan{} ) },
            OpType::Exponential => { Box::new( OpExponential{} ) },
            OpType::Log => { Box::new( OpLog{} ) },
            OpType::Sigmoid => { Box::new( OpSigmoid{} ) },
            // OpType::Softmax => { Box::new( OpSoftmax{} ) },
            OpType::Tanh => { Box::new( OpTanh{} ) },
            OpType::Relu => { Box::new( OpRelu{} ) },
            OpType::ReluLeaky => { Box::new( OpReluLeaky{} ) },
            // _ => { panic!( "unsupported op" ); },
        };
        l._op = b;
        let arg_ids : Vec< usize > = args.iter().map( |x| x._id ).collect();
        l.set_precedent( arg_ids.as_slice() );
        for i in args {
            (*i).set_descendent( & [ a ] );
        }
        l
    }
    ///computes dy/dx and other variables as well back propagating from y
    pub fn compute_grad( & mut self, y: usize, x: usize ) -> Result< Vec< f64 >, &'static str > {

        let index_y = *self._id_map.get(&y).unwrap();
        let index_x = *self._id_map.get(&x).unwrap();
        if self._is_evaluated ==  index_y {
            return Ok( self._buf[ index_x ]._grad.clone() )
        }

        //reset and do gradient compute starting at y
        self._is_evaluated = <usize>::max_value();

        // println!("eval order: {:?}", self._eval_order );
        
        let index_y = *self._id_map.get(&y).unwrap();

        // println!("y: {:?}", index_y );
        
        assert!( index_y < self._eval_order.len() );

        let ref mut link = & mut self._buf[..];

        //reset gradients of all variables
        for i in link.iter_mut() {
            for j in i._grad.iter_mut() {
                *j = 0f64;
            }
        }

        let index_y_order = *self._eval_order_map.get( & index_y ).unwrap();
        
        if self._eval_order.len() > 0 {
            for i in link[ self._eval_order[ index_y_order ] ]._grad.iter_mut() {
                *i = 1f64;
            }
        }
        for i in self._eval_order.iter() {
            //get values from precendent
            let g = {
                let mut params = vec![];
                for j in link[*i].get_precedent() {
                    let index_j = *self._id_map.get(j).unwrap();
                    let v = &link[index_j]._val[..];
                    params.push( v );
                }

                //compute backward gradient
                link[*i]._op.get_grad( params.as_slice() )
            };
            //g is a vector of vector of computed gradients of precedents

            assert!( g.len() == link[*i].get_precedent().len() );
            let mut index = 0;
            let v =  { link[*i].get_precedent().iter().cloned().collect::<Vec< usize > >() };

            // //accumulate gradients backward for precedents
            // for j in v { //for each precedent

            //     let index_j = *self._id_map.get(&j).unwrap();

            //     assert_eq!( g[index].len(), link[*i]._grad.len() );
            //     assert_eq!( g[index].len(), link[index_j]._grad.len() );

            //     for n in 0..g[index].len() { //for each scalar in gradient vector
            //         link[index_j]._grad[n] += g[index][n] * link[*i]._grad[n];
            //     }
            //     index += 1;
            // }

            //test--- replacement for the above commented code
            //accumulate gradients backward for precedents
            let mut accum_index = 0;
            for j in v { //for each precedent

                let index_j = *self._id_map.get(&j).unwrap();
                
                let n = g[index].len();

                //for each scalar in gradient vector
                let grad_delta = link[*i]._op.accum_grad_backward( & link[*i]._grad[..], & link[index_j]._grad[..], n, accum_index, &g[index][..] );

                accum_index += link[index_j]._grad.len();
                
                for (idx,hj) in link[index_j]._grad.iter_mut().enumerate(){
                    *hj += grad_delta[idx];
                }

                index += 1;
            }
        }

        self._is_evaluated = index_y;

        let ans = link[ index_x ]._grad.clone();
        Ok( ans )
    }
    ///checker for link validity, computes forward values, saves eval order for backward pass in context, and returns ids for input links
    pub fn fwd_pass( & mut self, mut link: Vec< Link > ) -> Result< Vec< usize >, &'static str > {
        self._is_evaluated = <usize>::max_value();

        //collect all leaf links that have no incoming dependencies
        let mut l : Vec< usize > = link.iter().enumerate().filter_map( |(_,x)| if x._precedent.len() == 0 { Some(x._id) } else { None } ).collect();
        
        // println!("collected leaves: {:?}", l );
        let mut checked = vec![ false; link.len() ];
        let mut temp : Vec< usize > = vec![];
        let mut eval_order = vec![];

        let mut ids = vec![];

        //map id to index in vec
        self._id_map.clear();
        self._eval_order.clear();
        self._eval_order_map.clear();

        for (e,i) in link.iter_mut().enumerate() {
            self._id_map.insert( i._id, e );
            ids.push( i._id );
        }

        //initilize gradient vector of leaf variables and checked
        for i in l.iter() {
            let index_i = *self._id_map.get(&i).unwrap();
            link[ index_i ]._grad = vec![ 0f64; link[index_i]._val.len() ];
            // println!("init grad: {:?}", link[ index_i ]._grad );

            checked[ index_i ] = true;
        }
        
        // println!("link.len: {:?}", link.len() );
        while l.len() > 0 || temp.len() > 0 {
            // println!("l.len: {:?}", l.len() );
            for i in l.iter() {
                // println!("checking: {}", i );

                let index_i = *self._id_map.get(i).unwrap();

                if checked[index_i] == false { //precedents not satisfied, delay it instead
                    temp.push( *i );
                    continue;
                }
                
                link[index_i].check()?;
                let ret = {

                    let mut max_param_size = 0usize;
                    let mut min_param_size = <usize>::max_value();
                    let mut precedent_val_len = vec![];

                    if link[index_i]._op.reshape_input() {
                        //presweep to determine if any scalar variable needs to be reshaped
                        for j in link[index_i].get_precedent() {
                            let index_j = *self._id_map.get(j).unwrap();
                            let param_len = link[index_j]._val.len();
                            max_param_size = cmp::max( max_param_size, param_len );
                            min_param_size = cmp::min( min_param_size, param_len );
                            precedent_val_len.push( ( index_j, param_len ) );
                        }

                        if min_param_size != max_param_size {
                            for j in precedent_val_len {
                                let index = j.0;
                                let current_len = j.1;
                                if current_len == max_param_size {
                                    continue;
                                } else if current_len == 1 {
                                    //reshape this scalar to a vector
                                    link[ index ]._val = {
                                        let v = link[index]._val[0];
                                        vec![ v; max_param_size ]
                                    };
                                    link[ index ]._grad = {
                                        let v = link[index]._grad[0];
                                        vec![ v; max_param_size ]
                                    };
                                } else {
                                    panic!( "variable length not consistent" )
                                }
                            }
                        }
                    }

                    //get values from precendent and compute forward val
                    let mut params = vec![];
                    for j in link[index_i].get_precedent() {
                        let index_j = *self._id_map.get(j).unwrap();
                        let v = link[index_j]._val.as_slice();
                        params.push(v);
                    }
                    link[index_i]._op.exec( params )
                };

                if ret.len() > 0 {
                    //store forward val
                    link[index_i]._val = ret;
                    //initilize gradient vector of non-leaf variables
                    if link[index_i]._grad.len() != link[index_i]._val.len() {
                        link[index_i]._grad = vec![ 0f64; link[index_i]._val.len() ];
                    }
                    // println!("init graident vector for node: {}, {:?}", index_i, link[index_i]._grad );
                }
                
                //queue descendents
                for j in link[index_i].get_descendent() {
                    let index_j = *self._id_map.get(j).unwrap();
                    if checked[index_j] == false {
                        //check that all precedents have already been processed
                        let fulfilled = link[index_j].get_precedent().iter().all(|pred_id|{
                            let index_pred = *self._id_map.get(pred_id).unwrap();
                            checked[index_pred]
                        });
                        if fulfilled {
                            temp.push(*j);
                            checked[index_j] = true;
                        } else {
                            temp.push(*j);
                        }
                    }
                }
                
                eval_order.push(index_i);
            }
            l = temp.drain(..).collect();
        }
        eval_order.reverse();
        
        self._buf = link.drain(..).collect();

        //save the forward order in terms of index of the input link
        self._eval_order = eval_order;

        for (e,v) in self._eval_order.iter().enumerate() {
            self._eval_order_map.insert(*v,e);
        }

        Ok( ids )
    }
}


#[derive(Clone, Debug)]
pub struct Link {
    //incoming nodes in the forward computation graph
    pub _precedent: Vec< usize >,
    //outgoing nodes in the forward computation graph
    pub _descendent: Vec< usize >,
    pub _op: Box< Op >,
    pub _val: Vec< f64 >,
    pub _grad: Vec< f64 >,
    pub _id: usize,
    
}

impl Clone for Box< Op > {
    fn clone( &self ) -> Box< Op > {
        self.box_clone()
    }
}

impl Default for Link {
    fn default() -> Link {
        Link {
            _precedent: vec![],
            _descendent: vec![],
            _op: Box::new( OpLeaf{} ), 
            _val: vec![],
            _grad: vec![],
            _id: 0usize,
        }
    }
}
impl Link {
    pub fn get_precedent( & self ) -> &[usize] {
        self._precedent.as_slice()
    }
    pub fn get_descendent( & self ) -> &[usize] {
        self._descendent.as_slice()
    }
    pub fn set_precedent(  & mut self, input: &[usize] ) {
        for i in input {
            self._precedent.push( *i );
        }
    }
    pub fn set_descendent( & mut self, input: &[usize] ) {
        for i in input {
            self._descendent.push( *i );
        }
    }
    pub fn clear_precedent( & mut self ) {
        self._precedent.clear();
    }
    pub fn clear_descendent( & mut self ) {
        self._descendent.clear();
    }
    pub fn check( & self ) -> Result< (), &'static str > {
        if self._op.get_arity() != self._precedent.len() {
            return Err( "op arity not match" )
        }
        Ok( () )
    }
    pub fn set_val( & mut self, val: &[f64] ) {
        match self._op {
            ref OpLeaf => {
                if val.len() != self._val.len() {
                    panic!("setting val array length not match");
                } else {
                    self._val = val.to_vec();
                }
            },
            _ => {
                panic!("cannot set value for nonleaf node");
            },
        }
    }
    pub fn get_val( & self ) -> &[f64] {
        &self._val[..]
    }
    pub fn get_val_mut( & mut self ) -> & mut [f64] {
        & mut self._val[..]
    }
}

///forward Op and gradient interface
pub trait Op : fmt::Debug {
    fn box_clone( & self ) -> Box< Op >;
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
    fn exec( & self, _: Vec< & [ f64 ] > ) -> Vec< f64 >;
    fn get_grad( & self, _: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > >;
    fn get_arity( & self ) -> usize;
    fn accum_grad_backward( & self, current_grad: & [f64], precedent_grad: & [f64], n: usize, accum_index: usize, calculated_gradient: &[f64] ) -> Vec<f64> {
        assert_eq!( calculated_gradient.len(), current_grad.len() );
        assert_eq!( calculated_gradient.len(), precedent_grad.len() );
        (0..n).map(|x| calculated_gradient[x] * current_grad[x] ).collect()
        // precedent_grad[n] += calculated_gradient[n] * current_grad[n];        
    }
    fn reshape_input( & self ) -> bool {
        true
    }
}
    
///y = constant; y' = 0;
#[derive(Clone, Debug)]
struct OpLeaf {}
impl Op for OpLeaf {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 0 );
        vec![]
    }
    fn get_arity( &self ) -> usize {
        0
    }
    fn exec( & self, _input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        vec![]
    }
}

///y = a_i; dy/da_i = 1
#[derive(Clone, Debug)]
pub struct OpConcat {
    _arity: usize,
}
impl Op for OpConcat {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: &[ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == self._arity );
        input.iter().map( |x| { vec![ 1.; x.len() ] } ).collect::<Vec<_>>()
    }
    fn get_arity( &self ) -> usize {
        self._arity
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == self._arity );
        let mut v = vec![];
        for i in input.iter() {
            for j in *i {
                v.push(*j);
            }
        }
        v
    }
    fn accum_grad_backward( & self, current_grad: & [f64], precedent_grad: & [f64], n: usize, accum_index: usize, calculated_gradient: &[f64] ) -> Vec<f64> {

        assert!( calculated_gradient.len() == precedent_grad.len() );
        assert!( current_grad.len() >= calculated_gradient.len() );
        
        let cur_g = current_grad.iter().skip(accum_index).take(calculated_gradient.len());
        let ret = cur_g.zip( calculated_gradient.iter() ).map( |(x,y)| *x * *y ).collect::<Vec<_>>();
        assert_eq!( ret.len(), calculated_gradient.len() );
        ret
    }
    fn reshape_input( & self ) -> bool {
        false
    }
}

///y = a*b; dy/da = b; dy/db = a
#[derive(Clone, Debug)]
pub struct OpMul {}
impl Op for OpMul {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: &[ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 2 );
        if input[0].len() == input[1].len() {
            vec![
                (*input[1]).to_vec(),
                (*input[0]).to_vec()
            ]
        } else if input[0].len() == 1 {
            vec![
                (*input[1]).to_vec(),
                vec![ input[0][0]; input[1].len() ]
            ]
        } else if input[1].len() == 1 {
            vec![
                vec![ input[1][0]; input[0].len() ],
                (*input[0]).to_vec(),
            ]
        } else {
            panic!( "argument size invalid" );
        }
    }
    fn get_arity( &self ) -> usize {
        2
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        (*input[0]).iter().zip( (*input[1]).iter() ).map( |x| x.0 * x.1 ).collect()
    }
}

///y = a/b; dy/da = 1/b; dy/db = -a/(b^2)
#[derive(Clone, Debug)]
pub struct OpDiv {}
impl Op for OpDiv {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: &[ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 2 );
        if input[0].len() == input[1].len() {
            vec![
                (*input[1]).iter().map(|x| 1./x).collect::<Vec<_>>(),
                (*input[0]).iter().zip((*input[1]).iter()).map(|(a,b)| -a/(b*b)).collect::<Vec<_>>()
            ]
        } else if input[0].len() == 1 {
            vec![
                (*input[1]).iter().map(|x| 1./x).collect::<Vec<_>>(),
                (*input[1]).iter().map(|x| -input[0][0]/(x*x)).collect::<Vec<_>>()
            ]
        } else if input[1].len() == 1 {
            vec![
                vec![ 1./input[1][0]; input[0].len() ],
                (*input[0]).iter().map(|x| -x/(input[1][0]*input[1][0])).collect::<Vec<_>>()
            ]
        } else {
            panic!( "argument size invalid" );
        }
    }
    fn get_arity( &self ) -> usize {
        2
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        (*input[0]).iter().zip( (*input[1]).iter() ).map( |x| x.0 / x.1 ).collect()
    }
}

///y = a + b; dy/da = 1; dy/db = 1;
#[derive(Clone, Debug)]
pub struct OpAddParallel {}
impl Op for OpAddParallel {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        vec![ vec![ 1f64; (*input[0]).len() ],
              vec![ 1f64; (*input[1]).len() ] ]
    }
    fn get_arity( &self ) -> usize {
        2
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        (*input[0]).iter().zip( (*input[1]).iter() ).map( |x| x.0 + x.1 ).collect()
    }
}

///y = sum{i}(a_i); dy/da_i = 1 for all i
#[derive(Clone, Debug)]
pub struct OpAddAll {}
impl Op for OpAddAll {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![ vec![ 1f64; (*input[0]).len() ] ]
    }
    fn get_arity( &self ) -> usize {
        1
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        vec![ (*input[0]).iter().fold( 0., |acc,x| acc+x ) ]
    }
    fn accum_grad_backward( & self, current_grad: & [f64], precedent_grad: & [f64], n: usize, accum_index: usize, calculated_gradient: &[f64] ) -> Vec<f64> {
        assert_eq!( current_grad.len(), 1 );
        assert_eq!( calculated_gradient.len(), precedent_grad.len() );
        calculated_gradient.iter().map(|x| x * current_grad[0] ).collect()
    }
}

///y = sin(x); dy/dx = cos(x)
#[derive(Clone, Debug)]
pub struct OpSin {}
impl Op for OpSin {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![ (*input[0]).iter().map( |x| x.cos() ).collect() ]
    }
    fn get_arity( &self ) -> usize {
        1
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| x.sin() ).collect()
    }
}

///y = cos(x); dy/dx = -sin(x)
#[derive(Clone, Debug)]
pub struct OpCos {}
impl Op for OpCos {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![ (*input[0]).iter().map( |x| -x.cos() ).collect() ]
    }
    fn get_arity( &self ) -> usize {
        1
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| x.cos() ).collect()
    }
}


///y = tan(x); dy/dx =  1/(cos(x))^2
#[derive(Clone, Debug)]
pub struct OpTan {}
impl Op for OpTan {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![ (*input[0]).iter().map( |x| 1f64 / ( x.cos().powf( 2f64 ) ) ).collect() ]
    }
    fn get_arity( &self ) -> usize {
        1
    }
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| x.tan() ).collect()
    }
}

///y = a^x; dy/dx = ln(a) * a^x
#[derive(Clone, Debug)]
pub struct OpExponential {}
impl Op for OpExponential {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    ///input[0]: bases, input[1]: exponents
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        vec![ vec![ 0f64; input[0].len()],
              (*input[0])
                .iter()
                .zip( (*input[1]).iter() )
                .map( |(base,exp)|
                        (*base).ln() * (*base).powf( *exp ) )
                .collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        2
    }
    ///input[0]: bases, input[1]: exponents
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        (*input[0]).iter().zip( (*input[1]).iter() ).map( |(base,exp)| (*base).powf(*exp) ).collect()
    }
}

///y = log_base(x); dy/dx = 1/(x*ln(base))
#[derive(Clone, Debug)]
pub struct OpLog {}
impl Op for OpLog {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }
    ///input[0]: bases, input[1]: nums
    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 2 );
        // vec![ 1f64 / ( (*input[1]) * (*input[0]).ln() ) ]
        vec![ vec![ 0f64; input[0].len()],
              (*input[0])
                .iter()
                .zip( (*input[1]).iter() )
                .map( |(base,num)|
                        1f64 / ( (*num) * (*base).ln() ) )
                .collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        2
    }
    ///input[0]: bases, input[1]: nums
    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 2 );
        assert!( input[0].len() == input[1].len() );
        (*input[0]).iter().zip( (*input[1]).iter() ).map( |(base,num)| (*num).log(*base) ).collect()
    }
}

///y = 1/(1+e^(-x)); dy/dx = y * ( 1 - y )
#[derive(Clone, Debug)]
pub struct OpSigmoid {}
impl Op for OpSigmoid {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }

    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![ (*input[0])
                .iter()
                .map( |x| {
                    let s = 1./( 1. + (-x).exp() );
                    s * ( 1. - s )
                })
                .collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        1
    }

    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| 1./( 1. + (-x).exp() ) ).collect()
    }
}

// ///y(x)_i = e^(-x_i)/sum{j}(e^(-x_j)); dy_i/dx_j = (y(x)_i)(kronecker_ik - (y(x)_i))
// #[derive(Clone, Debug)]
// pub struct OpSoftmax {}
// impl Op for OpSoftmax {

//     fn box_clone( & self ) -> Box< Op > {
//         Box::new( (*self).clone() )
//     }

//     fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{:?}", self )
//     }

//     fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
//         assert!( input.len() == 1 );
        
//         let d =
//             (*input[0])
//             .iter()
//             .fold( 0., |acc,x| {
//                 acc + (-x).exp()
//             });
            
//         vec![ (*input[0])
//                 .iter()
//                 .map( |x| {
//                     (-x).exp()/d
//                 })
//                 .collect()
//         ]
//     }

//     fn get_arity( &self ) -> usize {
//         1
//     }

//     fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
//         assert!( input.len() == 1 );
//         let d =
//             (*input[0])
//             .iter()
//             .fold( 0., |acc,x| {
//                 acc + (-x).exp()
//             });
            
//         (*input[0])
//             .iter()
//             .map( |x| {
//                 (-*x).exp()/d
//             })
//             .collect()
//     }
// }

///y = sinh(x)/cosh(x) = (1-e^(-2x))/(1+e^(-2x)); dy/dx = 1 - tanh(x)^2
#[derive(Clone, Debug)]
pub struct OpTanh {}
impl Op for OpTanh {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }

    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![
            (*input[0]).iter().map( |x| {
                let a = (-2.*x).exp();
                let b = ( 1. - a ) / ( 1. + a );
                1. - b.powi(2)
            } ).collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        1
    }

    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| { let a = (-2.*x).exp(); ( 1. - a ) / ( 1. + a ) } ).collect()
    }
}

///y = max(0,x); dy/dx = { 1 | x > 0, 0 | x < 0, 0 | x == 0 }
#[derive(Clone, Debug)]
pub struct OpRelu {}
impl Op for OpRelu {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self )
    }

    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![
            (*input[0]).iter().map( |x| if *x > 0. { 1. } else { 0. } ).collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        1
    }

    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| x.max( 0. ) ).collect()
    }
}

///y = max(0.01x,x); dy/dx = { 1 | x > 0, 0.01x | x <= 0 }
#[derive(Clone, Debug)]
pub struct OpReluLeaky {}
impl Op for OpReluLeaky {
    fn box_clone( & self ) -> Box< Op > {
        Box::new( (*self).clone() )
    }
    fn box_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!( f, "{:?}", self )
    }

    fn get_grad( &self, input: & [ & [ f64 ] ] ) -> Vec< Vec< f64 > > {
        assert!( input.len() == 1 );
        vec![
            (*input[0]).iter().map( |x| if *x > 0. { 1. } else { 0.01 } ).collect()
        ]
    }
    fn get_arity( &self ) -> usize {
        1
    }

    fn exec( & self, input: Vec< & [ f64 ] > ) -> Vec< f64 > {
        assert!( input.len() == 1 );
        (*input[0]).iter().map( |x| if *x > 0. { *x } else { 0.01 * x } ).collect()
    }
}
