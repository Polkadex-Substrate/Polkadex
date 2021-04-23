/// Emit binary machine code for `inst` for the x86 ISA.
#[allow(unused_variables, unreachable_code)]
pub fn emit_inst<CS: CodeSink + ?Sized>(
    func: &Function,
    inst: Inst,
    divert: &mut RegDiversions,
    sink: &mut CS,
    isa: &dyn TargetIsa,
) {
    let encoding = func.encodings[inst];
    let bits = encoding.bits();
    let inst_data = &func.dfg[inst];
    match encoding.recipe() {
        // Recipe get_pinned_reg
        0 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                return;
            }
        }
        // Recipe RexOp1set_pinned_reg
        1 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let r15 = RU::r15.into();
                put_rexop1(bits, rex2(r15, in_reg0), sink);
                modrm_rr(r15, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp1umr
        2 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexop1(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink);
                return;
            }
        }
        // Recipe RexOp1umr
        3 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink);
                return;
            }
        }
        // Recipe Op1umr
        4 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink);
                return;
            }
        }
        // Recipe Op1rmov
        5 => {
            if let InstructionData::RegMove {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                put_op1(bits, rex2(dst, src), sink);
                modrm_rr(dst, src, sink);
                return;
            }
        }
        // Recipe RexOp1rmov
        6 => {
            if let InstructionData::RegMove {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                put_rexop1(bits, rex2(dst, src), sink);
                modrm_rr(dst, src, sink);
                return;
            }
        }
        // Recipe Op1pu_id
        7 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1pu_id
        8 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1u_id
        9 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex1(out_reg0), sink);
                modrm_r_bits(out_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1pu_iq
        10 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                let imm: i64 = imm.into();
                sink.put8(imm as u64);
                return;
            }
        }
        // Recipe Op1pu_id_bool
        11 => {
            if let InstructionData::UnaryBool {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                let imm: u32 = if imm { 1 } else { 0 };
                sink.put4(imm);
                return;
            }
        }
        // Recipe RexOp1pu_id_bool
        12 => {
            if let InstructionData::UnaryBool {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                let imm: u32 = if imm { 1 } else { 0 };
                sink.put4(imm);
                return;
            }
        }
        // Recipe Op1u_id_z
        13 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp1u_id_z
        14 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe null
        15 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                return;
            }
        }
        // Recipe Op2urm_noflags_abcd
        16 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2urm_noflags
        17 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Op2urm_noflags
        18 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp1urm_noflags
        19 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp1copysp
        20 => {
            if let InstructionData::CopySpecial {
                opcode,
                src,
                dst,
                ..
            } = *inst_data {
                put_rexop1(bits, rex2(dst, src), sink);
                modrm_rr(dst, src, sink);
                return;
            }
        }
        // Recipe Op1copysp
        21 => {
            if let InstructionData::CopySpecial {
                opcode,
                src,
                dst,
                ..
            } = *inst_data {
                put_op1(bits, rex2(dst, src), sink);
                modrm_rr(dst, src, sink);
                return;
            }
        }
        // Recipe Op1umr_reg_to_ssa
        22 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex2(out_reg0, src), sink);
                modrm_rr(out_reg0, src, sink);
                return;
            }
        }
        // Recipe RexOp1umr_reg_to_ssa
        23 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(out_reg0, src), sink);
                modrm_rr(out_reg0, src, sink);
                return;
            }
        }
        // Recipe Mp2furm_reg_to_ssa
        24 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(src, out_reg0), sink);
                modrm_rr(src, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2furm_reg_to_ssa
        25 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(src, out_reg0), sink);
                modrm_rr(src, out_reg0, sink);
                return;
            }
        }
        // Recipe dummy_sarg_t
        26 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                return;
            }
        }
        // Recipe Op1ldWithIndex
        27 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp1ldWithIndex
        28 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op2ldWithIndex
        29 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp2ldWithIndex
        30 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op1ldWithIndexDisp8
        31 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp1ldWithIndexDisp8
        32 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2ldWithIndexDisp8
        33 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp2ldWithIndexDisp8
        34 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1ldWithIndexDisp32
        35 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp1ldWithIndexDisp32
        36 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2ldWithIndexDisp32
        37 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp2ldWithIndexDisp32
        38 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1stWithIndex
        39 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexOp1stWithIndex
        40 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Mp1stWithIndex
        41 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexMp1stWithIndex
        42 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Op1stWithIndexDisp8
        43 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp1stWithIndexDisp8
        44 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp1stWithIndexDisp8
        45 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp1stWithIndexDisp8
        46 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1stWithIndexDisp32
        47 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp1stWithIndexDisp32
        48 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp1stWithIndexDisp32
        49 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp1stWithIndexDisp32
        50 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1stWithIndex_abcd
        51 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexOp1stWithIndex_abcd
        52 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Op1stWithIndexDisp8_abcd
        53 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp1stWithIndexDisp8_abcd
        54 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1stWithIndexDisp32_abcd
        55 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp1stWithIndexDisp32_abcd
        56 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1st
        57 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp1st
        58 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp1st
        59 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexMp1st
        60 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op1stDisp8
        61 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp1stDisp8
        62 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp1stDisp8
        63 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp1stDisp8
        64 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1stDisp32
        65 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp1stDisp32
        66 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp1stDisp32
        67 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp1stDisp32
        68 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1st_abcd
        69 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op1stDisp8_abcd
        70 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1stDisp32_abcd
        71 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1spillSib32
        72 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_op1(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexOp1spillSib32
        73 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_rexop1(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe Op1regspill32
        74 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_op1(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe RexOp1regspill32
        75 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_rexop1(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe Op1ld
        76 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp1ld
        77 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op2ld
        78 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp2ld
        79 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op1ldDisp8
        80 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp1ldDisp8
        81 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2ldDisp8
        82 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp2ldDisp8
        83 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op1ldDisp32
        84 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp1ldDisp32
        85 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop1(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2ldDisp32
        86 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp2ldDisp32
        87 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op1fillSib32
        88 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_op1(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexOp1fillSib32
        89 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_rexop1(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe Op1regfill32
        90 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_op1(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe RexOp1regfill32
        91 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_rexop1(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe fillnull
        92 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                return;
            }
        }
        // Recipe ffillnull
        93 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                return;
            }
        }
        // Recipe Op1pushq
        94 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_op1(bits | (in_reg0 & 7), rex1(in_reg0), sink);
                return;
            }
        }
        // Recipe RexOp1pushq
        95 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_rexop1(bits | (in_reg0 & 7), rex1(in_reg0), sink);
                return;
            }
        }
        // Recipe Op1popq
        96 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                return;
            }
        }
        // Recipe RexOp1popq
        97 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                return;
            }
        }
        // Recipe stacknull
        98 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                return;
            }
        }
        // Recipe Op1adjustsp
        99 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_op1(bits, rex2(RU::rsp.into(), in_reg0), sink);
                modrm_rr(RU::rsp.into(), in_reg0, sink);
                return;
            }
        }
        // Recipe RexOp1adjustsp
        100 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex2(RU::rsp.into(), in_reg0), sink);
                modrm_rr(RU::rsp.into(), in_reg0, sink);
                return;
            }
        }
        // Recipe Op1adjustsp_ib
        101 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                put_op1(bits, rex1(RU::rsp.into()), sink);
                modrm_r_bits(RU::rsp.into(), bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe Op1adjustsp_id
        102 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                put_op1(bits, rex1(RU::rsp.into()), sink);
                modrm_r_bits(RU::rsp.into(), bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1adjustsp_ib
        103 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                put_rexop1(bits, rex1(RU::rsp.into()), sink);
                modrm_r_bits(RU::rsp.into(), bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe RexOp1adjustsp_id
        104 => {
            if let InstructionData::UnaryImm {
                opcode,
                imm,
                ..
            } = *inst_data {
                put_rexop1(bits, rex1(RU::rsp.into()), sink);
                modrm_r_bits(RU::rsp.into(), bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe Mp2frurm
        105 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2frurm
        106 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp2rfumr
        107 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2rfumr
        108 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink);
                return;
            }
        }
        // Recipe Op2furm
        109 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2furm
        110 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Op2frmov
        111 => {
            if let InstructionData::RegMove {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                put_op2(bits, rex2(src, dst), sink);
                modrm_rr(src, dst, sink);
                return;
            }
        }
        // Recipe RexOp2frmov
        112 => {
            if let InstructionData::RegMove {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                put_rexop2(bits, rex2(src, dst), sink);
                modrm_rr(src, dst, sink);
                return;
            }
        }
        // Recipe Mp2fld
        113 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexMp2fld
        114 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp2fldDisp8
        115 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp2fldDisp8
        116 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp2fldDisp32
        117 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp2fldDisp32
        118 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp2fldWithIndex
        119 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexMp2fldWithIndex
        120 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp2fldWithIndexDisp8
        121 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp2fldWithIndexDisp8
        122 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp2fldWithIndexDisp32
        123 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp2fldWithIndexDisp32
        124 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp2fst
        125 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexMp2fst
        126 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp2fstDisp8
        127 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp2fstDisp8
        128 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp2fstDisp32
        129 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp2fstDisp32
        130 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp2fstWithIndex
        131 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexMp2fstWithIndex
        132 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Mp2fstWithIndexDisp8
        133 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp2fstWithIndexDisp8
        134 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp2fstWithIndexDisp32
        135 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp2fstWithIndexDisp32
        136 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp2ffillSib32
        137 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_mp2(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexMp2ffillSib32
        138 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_rexmp2(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe Mp2fregfill32
        139 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_mp2(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe RexMp2fregfill32
        140 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_rexmp2(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe Mp2fspillSib32
        141 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_mp2(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexMp2fspillSib32
        142 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_rexmp2(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe Mp2fregspill32
        143 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_mp2(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe RexMp2fregspill32
        144 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_rexmp2(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe Op2f32imm_z
        145 => {
            if let InstructionData::UnaryIeee32 {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp2f64imm_z
        146 => {
            if let InstructionData::UnaryIeee64 {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2f32imm_z
        147 => {
            if let InstructionData::UnaryIeee32 {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2f64imm_z
        148 => {
            if let InstructionData::UnaryIeee64 {
                opcode,
                imm,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp2frurm
        149 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp2furm
        150 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2furm
        151 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp2rfurm
        152 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2rfurm
        153 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp3furmi_rnd
        154 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp3(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                sink.put1(match opcode {
                    Opcode::Nearest => 0b00,
                    Opcode::Floor => 0b01,
                    Opcode::Ceil => 0b10,
                    Opcode::Trunc => 0b11,
                    x => panic!("{} unexpected for furmi_rnd", opcode),
                });
                return;
            }
        }
        // Recipe RexMp3furmi_rnd
        155 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp3(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                sink.put1(match opcode {
                    Opcode::Nearest => 0b00,
                    Opcode::Floor => 0b01,
                    Opcode::Ceil => 0b10,
                    Opcode::Trunc => 0b11,
                    x => panic!("{} unexpected for furmi_rnd", opcode),
                });
                return;
            }
        }
        // Recipe Mp2fa
        156 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2fa
        157 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Op2fcscc
        158 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::FloatCC::*;
                let setcc = match cond {
                    Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
                    Unordered                  => 0x9a, // UN       => setp  (P=1)
                    OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
                    UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
                    GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
                    GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
                    UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
                    UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
                    Equal |                       // EQ
                    NotEqual |                    // UN|LT|GT
                    LessThan |                    // LT
                    LessThanOrEqual |             // LT|EQ
                    UnorderedOrGreaterThan |      // UN|GT
                    UnorderedOrGreaterThanOrEqual // UN|GT|EQ
                    => panic!("{} not supported by fcscc", cond),
                };
                sink.put1(0x0f);
                sink.put1(setcc);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp2fcscc
        159 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::FloatCC::*;
                let setcc = match cond {
                    Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
                    Unordered                  => 0x9a, // UN       => setp  (P=1)
                    OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
                    UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
                    GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
                    GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
                    UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
                    UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
                    Equal |                       // EQ
                    NotEqual |                    // UN|LT|GT
                    LessThan |                    // LT
                    LessThanOrEqual |             // LT|EQ
                    UnorderedOrGreaterThan |      // UN|GT
                    UnorderedOrGreaterThanOrEqual // UN|GT|EQ
                    => panic!("{} not supported by fcscc", cond),
                };
                sink.put1(0x0f);
                sink.put1(setcc);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe Mp2fcscc
        160 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::FloatCC::*;
                let setcc = match cond {
                    Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
                    Unordered                  => 0x9a, // UN       => setp  (P=1)
                    OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
                    UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
                    GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
                    GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
                    UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
                    UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
                    Equal |                       // EQ
                    NotEqual |                    // UN|LT|GT
                    LessThan |                    // LT
                    LessThanOrEqual |             // LT|EQ
                    UnorderedOrGreaterThan |      // UN|GT
                    UnorderedOrGreaterThanOrEqual // UN|GT|EQ
                    => panic!("{} not supported by fcscc", cond),
                };
                sink.put1(0x0f);
                sink.put1(setcc);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexMp2fcscc
        161 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::FloatCC::*;
                let setcc = match cond {
                    Ordered                    => 0x9b, // EQ|LT|GT => setnp (P=0)
                    Unordered                  => 0x9a, // UN       => setp  (P=1)
                    OrderedNotEqual            => 0x95, // LT|GT    => setne (Z=0),
                    UnorderedOrEqual           => 0x94, // UN|EQ    => sete  (Z=1)
                    GreaterThan                => 0x97, // GT       => seta  (C=0&Z=0)
                    GreaterThanOrEqual         => 0x93, // GT|EQ    => setae (C=0)
                    UnorderedOrLessThan        => 0x92, // UN|LT    => setb  (C=1)
                    UnorderedOrLessThanOrEqual => 0x96, // UN|LT|EQ => setbe (Z=1|C=1)
                    Equal |                       // EQ
                    NotEqual |                    // UN|LT|GT
                    LessThan |                    // LT
                    LessThanOrEqual |             // LT|EQ
                    UnorderedOrGreaterThan |      // UN|GT
                    UnorderedOrGreaterThanOrEqual // UN|GT|EQ
                    => panic!("{} not supported by fcscc", cond),
                };
                sink.put1(0x0f);
                sink.put1(setcc);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe Op2fcmp
        162 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2fcmp
        163 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Mp2fcmp
        164 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2fcmp
        165 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp1rr
        166 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp1rr
        167 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp1rout
        168 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp1rout
        169 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp1rin
        170 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp1rin
        171 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp1rio
        172 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp1rio
        173 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp1r_ib
        174 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe RexOp1r_ib
        175 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexOp1r_id
        176 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1r_id
        177 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe DynRexOp1ur
        178 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe RexOp1ur
        179 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe Op1ur
        180 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_op1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe Op1rr
        181 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_op1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp2rrx
        182 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2rrx
        183 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp1div
        184 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg2 = divert.reg(args[2], &func.locations);
                sink.trap(TrapCode::IntegerDivisionByZero, func.srclocs[inst]);
                put_dynrexop1(bits, rex1(in_reg2), sink);
                modrm_r_bits(in_reg2, bits, sink);
                return;
            }
        }
        // Recipe RexOp1div
        185 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg2 = divert.reg(args[2], &func.locations);
                sink.trap(TrapCode::IntegerDivisionByZero, func.srclocs[inst]);
                put_rexop1(bits, rex1(in_reg2), sink);
                modrm_r_bits(in_reg2, bits, sink);
                return;
            }
        }
        // Recipe DynRexOp1mulx
        186 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex1(in_reg1), sink);
                modrm_r_bits(in_reg1, bits, sink);
                return;
            }
        }
        // Recipe RexOp1mulx
        187 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex1(in_reg1), sink);
                modrm_r_bits(in_reg1, bits, sink);
                return;
            }
        }
        // Recipe Op2fa
        188 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2fa
        189 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Op2fax
        190 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_op2(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp2fax
        191 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop2(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe Op1rc
        192 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_op1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe RexOp1rc
        193 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe Mp2urm
        194 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexMp2urm
        195 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp2bsf_and_bsr
        196 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = func.dfg.inst_results(inst);
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexop2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2bsf_and_bsr
        197 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = func.dfg.inst_results(inst);
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp1icscc
        198 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp1icscc
        199 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe DynRexOp1icscc_ib
        200 => {
            if let InstructionData::IntCompareImm {
                opcode,
                cond,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp1icscc_ib
        201 => {
            if let InstructionData::IntCompareImm {
                opcode,
                cond,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe DynRexOp1icscc_id
        202 => {
            if let InstructionData::IntCompareImm {
                opcode,
                cond,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp1icscc_id
        203 => {
            if let InstructionData::IntCompareImm {
                opcode,
                cond,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                // `setCC` instruction, no REX.
                let setcc = 0x90 | icc2opc(cond);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe DynRexOp1rcmp
        204 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe RexOp1rcmp
        205 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexop1(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexOp1rcmp_ib
        206 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe RexOp1rcmp_ib
        207 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexOp1rcmp_id
        208 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe RexOp1rcmp_id
        209 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put4(imm as u32);
                return;
            }
        }
        // Recipe Op1rcmp_sp
        210 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_op1(bits, rex2(in_reg0, RU::rsp.into()), sink);
                modrm_rr(in_reg0, RU::rsp.into(), sink);
                return;
            }
        }
        // Recipe RexOp1rcmp_sp
        211 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex2(in_reg0, RU::rsp.into()), sink);
                modrm_rr(in_reg0, RU::rsp.into(), sink);
                return;
            }
        }
        // Recipe Op2seti_abcd
        212 => {
            if let InstructionData::IntCond {
                opcode,
                cond,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits | icc2opc(cond), rex1(out_reg0), sink);
                modrm_r_bits(out_reg0, bits, sink);
                return;
            }
        }
        // Recipe RexOp2seti
        213 => {
            if let InstructionData::IntCond {
                opcode,
                cond,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits | icc2opc(cond), rex1(out_reg0), sink);
                modrm_r_bits(out_reg0, bits, sink);
                return;
            }
        }
        // Recipe Op2setf_abcd
        214 => {
            if let InstructionData::FloatCond {
                opcode,
                cond,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits | fcc2opc(cond), rex1(out_reg0), sink);
                modrm_r_bits(out_reg0, bits, sink);
                return;
            }
        }
        // Recipe RexOp2setf
        215 => {
            if let InstructionData::FloatCond {
                opcode,
                cond,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits | fcc2opc(cond), rex1(out_reg0), sink);
                modrm_r_bits(out_reg0, bits, sink);
                return;
            }
        }
        // Recipe DynRexOp2cmov
        216 => {
            if let InstructionData::IntSelect {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                put_dynrexop2(bits | icc2opc(cond), rex2(in_reg1, in_reg2), sink);
                modrm_rr(in_reg1, in_reg2, sink);
                return;
            }
        }
        // Recipe RexOp2cmov
        217 => {
            if let InstructionData::IntSelect {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                put_rexop2(bits | icc2opc(cond), rex2(in_reg1, in_reg2), sink);
                modrm_rr(in_reg1, in_reg2, sink);
                return;
            }
        }
        // Recipe Mp3fa
        218 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp3fa
        219 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Mp2r_ib_unsigned_fpr
        220 => {
            if let InstructionData::BinaryImm8 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp2r_ib_unsigned_fpr
        221 => {
            if let InstructionData::BinaryImm8 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe Mp3blend
        222 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                put_mp3(bits, rex2(in_reg1, in_reg2), sink);
                modrm_rr(in_reg1, in_reg2, sink);
                return;
            }
        }
        // Recipe DynRexMp3blend
        223 => {
            if let InstructionData::Ternary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg1, in_reg2), sink);
                modrm_rr(in_reg1, in_reg2, sink);
                return;
            }
        }
        // Recipe Mp3fa_ib
        224 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp3fa_ib
        225 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe null_fpr
        226 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                return;
            }
        }
        // Recipe Mp3r_ib_unsigned_r
        227 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp3r_ib_unsigned_r
        228 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe Mp2r_ib_unsigned_r
        229 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp2r_ib_unsigned_r
        230 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe RexMp3r_ib_unsigned_r
        231 => {
            if let InstructionData::TernaryImm8 {
                opcode,
                imm,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_rexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp2fa
        232 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexOp2fa
        233 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Mp3r_ib_unsigned_gpr
        234 => {
            if let InstructionData::BinaryImm8 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp3(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink); // note the flipped register in the ModR/M byte
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp3r_ib_unsigned_gpr
        235 => {
            if let InstructionData::BinaryImm8 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp3(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink); // note the flipped register in the ModR/M byte
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe RexMp3r_ib_unsigned_gpr
        236 => {
            if let InstructionData::BinaryImm8 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexmp3(bits, rex2(out_reg0, in_reg0), sink);
                modrm_rr(out_reg0, in_reg0, sink); // note the flipped register in the ModR/M byte
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe Mp3furm
        237 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_mp3(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp3furm
        238 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe EvexMp2evex_reg_rm_128
        239 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // instruction encoding operands: reg (op1, w), rm (op2, r)
                // this maps to:                  out_reg0,     in_reg0
                let context = EvexContext::Other { length: EvexVectorLength::V128 };
                let masking = EvexMasking::None;
                put_evex(bits, out_reg0, 0, in_reg0, context, masking, sink); // params: reg, vvvv, rm
                modrm_rr(in_reg0, out_reg0, sink); // params: rm, reg
                return;
            }
        }
        // Recipe DynRexMp2furm
        240 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp2vconst_optimized
        241 => {
            if let InstructionData::UnaryConst {
                opcode,
                constant_handle,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexmp2(bits, rex2(out_reg0, out_reg0), sink);
                modrm_rr(out_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Op2vconst
        242 => {
            if let InstructionData::UnaryConst {
                opcode,
                constant_handle,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                const_disp4(constant_handle, func, sink);
                return;
            }
        }
        // Recipe DynRexOp2vconst
        243 => {
            if let InstructionData::UnaryConst {
                opcode,
                constant_handle,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexop2(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                const_disp4(constant_handle, func, sink);
                return;
            }
        }
        // Recipe Op2fst
        244 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe DynRexOp2fst
        245 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else if needs_offset(in_reg1) {
                    modrm_disp8(in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op2fstDisp8
        246 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe DynRexOp2fstDisp8
        247 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp8(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2fstDisp32
        248 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe DynRexOp2fstDisp32
        249 => {
            if let InstructionData::Store {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                if needs_sib_byte(in_reg1) {
                    modrm_sib_disp32(in_reg0, sink);
                    sib_noindex(in_reg1, sink);
                } else {
                    modrm_disp32(in_reg1, in_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2fstWithIndex
        250 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexOp2fstWithIndex
        251 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(in_reg0, sink);
                    sib(0, in_reg2, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Op2fstWithIndexDisp8
        252 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp2fstWithIndexDisp8
        253 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp8(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2fstWithIndexDisp32
        254 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp2fstWithIndexDisp32
        255 => {
            if let InstructionData::StoreComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let in_reg2 = divert.reg(args[2], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg1, in_reg0, in_reg2), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib(0, in_reg2, in_reg1, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2fld
        256 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe DynRexOp2fld
        257 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op2fldDisp8
        258 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe DynRexOp2fldDisp8
        259 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2fldDisp32
        260 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe DynRexOp2fldDisp32
        261 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexop2(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2fldWithIndex
        262 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexOp2fldWithIndex
        263 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Op2fldWithIndexDisp8
        264 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexOp2fldWithIndexDisp8
        265 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Op2fldWithIndexDisp32
        266 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_op2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexOp2fldWithIndexDisp32
        267 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexop2(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Op2fspillSib32
        268 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_op2(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexOp2fspillSib32
        269 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_stk0 = StackRef::masked(
                    divert.stack(results[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let base = stk_base(out_stk0.base);
                put_rexop2(bits, rex2(base, in_reg0), sink);
                modrm_sib_disp32(in_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(out_stk0.offset as u32);
                return;
            }
        }
        // Recipe Op2fregspill32
        270 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_op2(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe RexOp2fregspill32
        271 => {
            if let InstructionData::RegSpill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                let dst = StackRef::sp(dst, &func.stack_slots);
                let base = stk_base(dst.base);
                put_rexop2(bits, rex2(base, src), sink);
                modrm_sib_disp32(src, sink);
                sib_noindex(base, sink);
                sink.put4(dst.offset as u32);
                return;
            }
        }
        // Recipe Op2ffillSib32
        272 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_op2(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe RexOp2ffillSib32
        273 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_stk0 = StackRef::masked(
                    divert.stack(args[0], &func.locations),
                    StackBaseMask(1),
                    &func.stack_slots,
                ).unwrap();
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let base = stk_base(in_stk0.base);
                put_rexop2(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                sink.put4(in_stk0.offset as u32);
                return;
            }
        }
        // Recipe Op2fregfill32
        274 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_op2(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe RexOp2fregfill32
        275 => {
            if let InstructionData::RegFill {
                opcode,
                src,
                dst,
                arg,
                ..
            } = *inst_data {
                divert.apply(inst_data);
                let src = StackRef::sp(src, &func.stack_slots);
                let base = stk_base(src.base);
                put_rexop2(bits, rex2(base, dst), sink);
                modrm_sib_disp32(dst, sink);
                sib_noindex(base, sink);
                sink.put4(src.offset as u32);
                return;
            }
        }
        // Recipe Op2furm_reg_to_ssa
        276 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op2(bits, rex2(src, out_reg0), sink);
                modrm_rr(src, out_reg0, sink);
                return;
            }
        }
        // Recipe RexOp2furm_reg_to_ssa
        277 => {
            if let InstructionData::CopyToSsa {
                opcode,
                src,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop2(bits, rex2(src, out_reg0), sink);
                modrm_rr(src, out_reg0, sink);
                return;
            }
        }
        // Recipe Mp3fld
        278 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe DynRexMp3fld
        279 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexmp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else if needs_offset(in_reg0) {
                    modrm_disp8(in_reg0, out_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_rm(in_reg0, out_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp3fldDisp8
        280 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe DynRexMp3fldDisp8
        281 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexmp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp8(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp3fldDisp32
        282 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe DynRexMp3fldDisp32
        283 => {
            if let InstructionData::Load {
                opcode,
                flags,
                offset,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_dynrexmp3(bits, rex2(in_reg0, out_reg0), sink);
                if needs_sib_byte(in_reg0) {
                    modrm_sib_disp32(out_reg0, sink);
                    sib_noindex(in_reg0, sink);
                } else {
                    modrm_disp32(in_reg0, out_reg0, sink);
                }
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe Mp3fldWithIndex
        284 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe RexMp3fldWithIndex
        285 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                // The else branch always inserts an SIB byte.
                if needs_offset(in_reg0) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(0, in_reg1, in_reg0, sink);
                }
                return;
            }
        }
        // Recipe Mp3fldWithIndexDisp8
        286 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe RexMp3fldWithIndexDisp8
        287 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp8(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put1(offset as u8);
                return;
            }
        }
        // Recipe Mp3fldWithIndexDisp32
        288 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_mp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe RexMp3fldWithIndexDisp32
        289 => {
            if let InstructionData::LoadComplex {
                opcode,
                flags,
                offset,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                if !flags.notrap() {
                    sink.trap(TrapCode::HeapOutOfBounds, func.srclocs[inst]);
                }
                put_rexmp3(bits, rex3(in_reg0, out_reg0, in_reg1), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib(0, in_reg1, in_reg0, sink);
                let offset: i32 = offset.into();
                sink.put4(offset as u32);
                return;
            }
        }
        // Recipe EvexMp3evex_reg_vvvv_rm_128
        290 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // instruction encoding operands: reg (op1, w), vvvv (op2, r), rm (op3, r)
                // this maps to:                  out_reg0,     in_reg0,       in_reg1
                let context = EvexContext::Other { length: EvexVectorLength::V128 };
                let masking = EvexMasking::None;
                put_evex(bits, out_reg0, in_reg0, in_reg1, context, masking, sink); // params: reg, vvvv, rm
                modrm_rr(in_reg1, out_reg0, sink); // params: rm, reg
                return;
            }
        }
        // Recipe Mp2fax
        291 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp2(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe DynRexMp2fax
        292 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp2(bits, rex2(in_reg0, in_reg1), sink);
                modrm_rr(in_reg0, in_reg1, sink);
                return;
            }
        }
        // Recipe Mp3fcmp
        293 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_mp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp3fcmp
        294 => {
            if let InstructionData::Binary {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                put_dynrexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Mp2f_ib
        295 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_mp2(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe DynRexMp2f_ib
        296 => {
            if let InstructionData::BinaryImm64 {
                opcode,
                imm,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_dynrexmp2(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                let imm: i64 = imm.into();
                sink.put1(imm as u8);
                return;
            }
        }
        // Recipe Mp2icscc_fpr
        297 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp2icscc_fpr
        298 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_dynrexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Mp3icscc_fpr
        299 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_mp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe DynRexMp3icscc_fpr
        300 => {
            if let InstructionData::IntCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_dynrexmp3(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                return;
            }
        }
        // Recipe Op2pfcmp
        301 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_op2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // Add immediate byte indicating what type of comparison.
                use crate::ir::condcodes::FloatCC::*;
                let imm = match cond {
                    Equal                      => 0x00,
                    LessThan                   => 0x01,
                    LessThanOrEqual            => 0x02,
                    Unordered                  => 0x03,
                    NotEqual                   => 0x04,
                    UnorderedOrGreaterThanOrEqual => 0x05,
                    UnorderedOrGreaterThan => 0x06,
                    Ordered                    => 0x07,
                    _ => panic!("{} not supported by pfcmp", cond),
                };
                sink.put1(imm);
                return;
            }
        }
        // Recipe DynRexOp2pfcmp
        302 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_dynrexop2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // Add immediate byte indicating what type of comparison.
                use crate::ir::condcodes::FloatCC::*;
                let imm = match cond {
                    Equal                      => 0x00,
                    LessThan                   => 0x01,
                    LessThanOrEqual            => 0x02,
                    Unordered                  => 0x03,
                    NotEqual                   => 0x04,
                    UnorderedOrGreaterThanOrEqual => 0x05,
                    UnorderedOrGreaterThan => 0x06,
                    Ordered                    => 0x07,
                    _ => panic!("{} not supported by pfcmp", cond),
                };
                sink.put1(imm);
                return;
            }
        }
        // Recipe Mp2pfcmp
        303 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_mp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // Add immediate byte indicating what type of comparison.
                use crate::ir::condcodes::FloatCC::*;
                let imm = match cond {
                    Equal                      => 0x00,
                    LessThan                   => 0x01,
                    LessThanOrEqual            => 0x02,
                    Unordered                  => 0x03,
                    NotEqual                   => 0x04,
                    UnorderedOrGreaterThanOrEqual => 0x05,
                    UnorderedOrGreaterThan => 0x06,
                    Ordered                    => 0x07,
                    _ => panic!("{} not supported by pfcmp", cond),
                };
                sink.put1(imm);
                return;
            }
        }
        // Recipe DynRexMp2pfcmp
        304 => {
            if let InstructionData::FloatCompare {
                opcode,
                cond,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                // Comparison instruction.
                put_dynrexmp2(bits, rex2(in_reg1, in_reg0), sink);
                modrm_rr(in_reg1, in_reg0, sink);
                // Add immediate byte indicating what type of comparison.
                use crate::ir::condcodes::FloatCC::*;
                let imm = match cond {
                    Equal                      => 0x00,
                    LessThan                   => 0x01,
                    LessThanOrEqual            => 0x02,
                    Unordered                  => 0x03,
                    NotEqual                   => 0x04,
                    UnorderedOrGreaterThanOrEqual => 0x05,
                    UnorderedOrGreaterThan => 0x06,
                    Ordered                    => 0x07,
                    _ => panic!("{} not supported by pfcmp", cond),
                };
                sink.put1(imm);
                return;
            }
        }
        // Recipe DynRexOp2furm
        305 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_dynrexop2(bits, rex2(in_reg0, out_reg0), sink);
                modrm_rr(in_reg0, out_reg0, sink);
                return;
            }
        }
        // Recipe Op1fnaddr4
        306 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    0);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1fnaddr8
        307 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs8,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    0);
                sink.put8(0);
                return;
            }
        }
        // Recipe Op1allones_fnaddr4
        308 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    0);
                // Write the immediate as `!0` for the benefit of BaldrMonkey.
                sink.put4(!0);
                return;
            }
        }
        // Recipe RexOp1allones_fnaddr8
        309 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs8,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    0);
                // Write the immediate as `!0` for the benefit of BaldrMonkey.
                sink.put8(!0);
                return;
            }
        }
        // Recipe RexOp1pcrel_fnaddr8
        310 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                // The addend adjusts for the difference between the end of the
                // instruction and the beginning of the immediate field.
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86PCRel4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    -4);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1got_fnaddr8
        311 => {
            if let InstructionData::FuncAddr {
                opcode,
                func_ref,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                // The addend adjusts for the difference between the end of the
                // instruction and the beginning of the immediate field.
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86GOTPCRel4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    -4);
                sink.put4(0);
                return;
            }
        }
        // Recipe Op1gvaddr4
        312 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs4,
                                    &func.global_values[global_value].symbol_name(),
                                    0);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1gvaddr8
        313 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::Abs8,
                                    &func.global_values[global_value].symbol_name(),
                                    0);
                sink.put8(0);
                return;
            }
        }
        // Recipe RexOp1pcrel_gvaddr8
        314 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_rm(5, out_reg0, sink);
                // The addend adjusts for the difference between the end of the
                // instruction and the beginning of the immediate field.
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86PCRel4,
                                    &func.global_values[global_value].symbol_name(),
                                    -4);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1got_gvaddr8
        315 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_rm(5, out_reg0, sink);
                // The addend adjusts for the difference between the end of the
                // instruction and the beginning of the immediate field.
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86GOTPCRel4,
                                    &func.global_values[global_value].symbol_name(),
                                    -4);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1spaddr_id
        316 => {
            if let InstructionData::StackLoad {
                opcode,
                stack_slot,
                offset,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let sp = StackRef::sp(stack_slot, &func.stack_slots);
                let base = stk_base(sp.base);
                put_rexop1(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                let imm : i32 = offset.into();
                sink.put4(sp.offset.checked_add(imm).unwrap() as u32);
                return;
            }
        }
        // Recipe Op1spaddr_id
        317 => {
            if let InstructionData::StackLoad {
                opcode,
                stack_slot,
                offset,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                let sp = StackRef::sp(stack_slot, &func.stack_slots);
                let base = stk_base(sp.base);
                put_op1(bits, rex2(base, out_reg0), sink);
                modrm_sib_disp32(out_reg0, sink);
                sib_noindex(base, sink);
                let imm : i32 = offset.into();
                sink.put4(sp.offset.checked_add(imm).unwrap() as u32);
                return;
            }
        }
        // Recipe RexOp1const_addr
        318 => {
            if let InstructionData::UnaryConst {
                opcode,
                constant_handle,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                const_disp4(constant_handle, func, sink);
                return;
            }
        }
        // Recipe Op1const_addr
        319 => {
            if let InstructionData::UnaryConst {
                opcode,
                constant_handle,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                const_disp4(constant_handle, func, sink);
                return;
            }
        }
        // Recipe Op1call_id
        320 => {
            if let InstructionData::Call {
                opcode,
                func_ref,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_op1(bits, BASE_REX, sink);
                // The addend adjusts for the difference between the end of the
                // instruction and the beginning of the immediate field.
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86CallPCRel4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    -4);
                sink.put4(0);
                sink.add_call_site(opcode, func.srclocs[inst]);
                return;
            }
        }
        // Recipe Op1call_plt_id
        321 => {
            if let InstructionData::Call {
                opcode,
                func_ref,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_op1(bits, BASE_REX, sink);
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86CallPLTRel4,
                                    &func.dfg.ext_funcs[func_ref].name,
                                    -4);
                sink.put4(0);
                sink.add_call_site(opcode, func.srclocs[inst]);
                return;
            }
        }
        // Recipe Op1call_r
        322 => {
            if let InstructionData::CallIndirect {
                opcode,
                sig_ref,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_op1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                sink.add_call_site(opcode, func.srclocs[inst]);
                return;
            }
        }
        // Recipe RexOp1call_r
        323 => {
            if let InstructionData::CallIndirect {
                opcode,
                sig_ref,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                sink.trap(TrapCode::StackOverflow, func.srclocs[inst]);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                sink.add_call_site(opcode, func.srclocs[inst]);
                return;
            }
        }
        // Recipe Op1ret
        324 => {
            if let InstructionData::MultiAry {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op1(bits, BASE_REX, sink);
                return;
            }
        }
        // Recipe Op1jmpb
        325 => {
            if let InstructionData::Jump {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op1(bits, BASE_REX, sink);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe Op1jmpd
        326 => {
            if let InstructionData::Jump {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op1(bits, BASE_REX, sink);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe Op1brib
        327 => {
            if let InstructionData::BranchInt {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op1(bits | icc2opc(cond), BASE_REX, sink);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1brib
        328 => {
            if let InstructionData::BranchInt {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_rexop1(bits | icc2opc(cond), BASE_REX, sink);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe Op2brid
        329 => {
            if let InstructionData::BranchInt {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op2(bits | icc2opc(cond), BASE_REX, sink);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp2brid
        330 => {
            if let InstructionData::BranchInt {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_rexop2(bits | icc2opc(cond), BASE_REX, sink);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe Op1brfb
        331 => {
            if let InstructionData::BranchFloat {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op1(bits | fcc2opc(cond), BASE_REX, sink);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1brfb
        332 => {
            if let InstructionData::BranchFloat {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_rexop1(bits | fcc2opc(cond), BASE_REX, sink);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe Op2brfd
        333 => {
            if let InstructionData::BranchFloat {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_op2(bits | fcc2opc(cond), BASE_REX, sink);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp2brfd
        334 => {
            if let InstructionData::BranchFloat {
                opcode,
                cond,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                put_rexop2(bits | fcc2opc(cond), BASE_REX, sink);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe Op1tjccb
        335 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test r, r.
                put_op1((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(bits as u8);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1tjccb
        336 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test r, r.
                put_rexop1((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(bits as u8);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe Op1tjccd
        337 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test r, r.
                put_op1((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(0x0f);
                sink.put1(bits as u8);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1tjccd
        338 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test r, r.
                put_rexop1((bits & 0xff00) | 0x85, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(0x0f);
                sink.put1(bits as u8);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe Op1t8jccd_long
        339 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test32 r, 0xff.
                put_op1((bits & 0xff00) | 0xf7, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                sink.put4(0xff);
                // Jcc instruction.
                sink.put1(0x0f);
                sink.put1(bits as u8);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe Op1t8jccb_abcd
        340 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test8 r, r.
                put_op1((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(bits as u8);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1t8jccb
        341 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test8 r, r.
                put_rexop1((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(bits as u8);
                disp1(destination, func, sink);
                return;
            }
        }
        // Recipe Op1t8jccd_abcd
        342 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test8 r, r.
                put_op1((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(0x0f);
                sink.put1(bits as u8);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1t8jccd
        343 => {
            if let InstructionData::Branch {
                opcode,
                destination,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                let in_reg0 = divert.reg(args[0], &func.locations);
                // test8 r, r.
                put_rexop1((bits & 0xff00) | 0x84, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Jcc instruction.
                sink.put1(0x0f);
                sink.put1(bits as u8);
                disp4(destination, func, sink);
                return;
            }
        }
        // Recipe RexOp1jt_entry
        344 => {
            if let InstructionData::BranchTableEntry {
                opcode,
                imm,
                table,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex3(in_reg1, out_reg0, in_reg0), sink);
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe Op1jt_entry
        345 => {
            if let InstructionData::BranchTableEntry {
                opcode,
                imm,
                table,
                ref args,
                ..
            } = *inst_data {
                let in_reg0 = divert.reg(args[0], &func.locations);
                let in_reg1 = divert.reg(args[1], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex3(in_reg1, out_reg0, in_reg0), sink);
                if needs_offset(in_reg1) {
                    modrm_sib_disp8(out_reg0, sink);
                    sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                    sink.put1(0);
                } else {
                    modrm_sib(out_reg0, sink);
                    sib(imm.trailing_zeros() as u8, in_reg0, in_reg1, sink);
                }
                return;
            }
        }
        // Recipe RexOp1jt_base
        346 => {
            if let InstructionData::BranchTableBase {
                opcode,
                table,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_rexop1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                
                // No reloc is needed here as the jump table is emitted directly after
                // the function body.
                jt_disp4(table, func, sink);
                return;
            }
        }
        // Recipe Op1jt_base
        347 => {
            if let InstructionData::BranchTableBase {
                opcode,
                table,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                put_op1(bits, rex2(0, out_reg0), sink);
                modrm_riprel(out_reg0, sink);
                
                // No reloc is needed here as the jump table is emitted directly after
                // the function body.
                jt_disp4(table, func, sink);
                return;
            }
        }
        // Recipe RexOp1indirect_jmp
        348 => {
            if let InstructionData::IndirectJump {
                opcode,
                table,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe Op1indirect_jmp
        349 => {
            if let InstructionData::IndirectJump {
                opcode,
                table,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                put_op1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                return;
            }
        }
        // Recipe Op2trap
        350 => {
            if let InstructionData::Trap {
                opcode,
                code,
                ..
            } = *inst_data {
                sink.trap(code, func.srclocs[inst]);
                put_op2(bits, BASE_REX, sink);
                return;
            }
        }
        // Recipe debugtrap
        351 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                sink.put1(0xcc);
                return;
            }
        }
        // Recipe trapif
        352 => {
            if let InstructionData::IntCondTrap {
                opcode,
                cond,
                code,
                ..
            } = *inst_data {
                // Jump over a 2-byte ud2.
                sink.put1(0x70 | (icc2opc(cond.inverse()) as u8));
                sink.put1(2);
                // ud2.
                sink.trap(code, func.srclocs[inst]);
                sink.put1(0x0f);
                sink.put1(0x0b);
                return;
            }
        }
        // Recipe trapff
        353 => {
            if let InstructionData::FloatCondTrap {
                opcode,
                cond,
                code,
                ..
            } = *inst_data {
                // Jump over a 2-byte ud2.
                sink.put1(0x70 | (fcc2opc(cond.inverse()) as u8));
                sink.put1(2);
                // ud2.
                sink.trap(code, func.srclocs[inst]);
                sink.put1(0x0f);
                sink.put1(0x0b);
                return;
            }
        }
        // Recipe Op1pu_id_ref
        354 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_op1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.put4(0);
                return;
            }
        }
        // Recipe RexOp1pu_id_ref
        355 => {
            if let InstructionData::NullAry {
                opcode,
                ..
            } = *inst_data {
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // The destination register is encoded in the low bits of the opcode.
                // No ModR/M.
                put_rexop1(bits | (out_reg0 & 7), rex1(out_reg0), sink);
                sink.put4(0);
                return;
            }
        }
        // Recipe Op1is_zero
        356 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Test instruction.
                put_op1(bits, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Check ZF = 1 flag to see if register holds 0.
                sink.put1(0x0f);
                sink.put1(0x94);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp1is_zero
        357 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Test instruction.
                put_rexop1(bits, rex2(in_reg0, in_reg0), sink);
                modrm_rr(in_reg0, in_reg0, sink);
                // Check ZF = 1 flag to see if register holds 0.
                sink.put1(0x0f);
                sink.put1(0x94);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe Op1is_invalid
        358 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_op1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                sink.put1(0xff);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::IntCC::*;
                let setcc = 0x90 | icc2opc(Equal);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe RexOp1is_invalid
        359 => {
            if let InstructionData::Unary {
                opcode,
                arg,
                ..
            } = *inst_data {
                let args = [arg];
                let in_reg0 = divert.reg(args[0], &func.locations);
                let results = [func.dfg.first_result(inst)];
                let out_reg0 = divert.reg(results[0], &func.locations);
                // Comparison instruction.
                put_rexop1(bits, rex1(in_reg0), sink);
                modrm_r_bits(in_reg0, bits, sink);
                sink.put1(0xff);
                // `setCC` instruction, no REX.
                use crate::ir::condcodes::IntCC::*;
                let setcc = 0x90 | icc2opc(Equal);
                sink.put1(0x0f);
                sink.put1(setcc as u8);
                modrm_rr(out_reg0, 0, sink);
                return;
            }
        }
        // Recipe safepoint
        360 => {
            if let InstructionData::MultiAry {
                opcode,
                ref args,
                ..
            } = *inst_data {
                let args = args.as_slice(&func.dfg.value_lists);
                sink.add_stack_map(args, func, isa);
                return;
            }
        }
        // Recipe elf_tls_get_addr
        361 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                // output %rax
                // clobbers %rdi
                
                // Those data16 prefixes are necessary to pad to 16 bytes.
                
                // data16 lea gv@tlsgd(%rip),%rdi
                sink.put1(0x66); // data16
                sink.put1(0b01001000); // rex.w
                const LEA: u8 = 0x8d;
                sink.put1(LEA); // lea
                modrm_riprel(0b111/*out_reg0*/, sink); // 0x3d
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::ElfX86_64TlsGd,
                                    &func.global_values[global_value].symbol_name(),
                                    -4);
                sink.put4(0);
                
                // data16 data16 callq __tls_get_addr-4
                sink.put1(0x66); // data16
                sink.put1(0x66); // data16
                sink.put1(0b01001000); // rex.w
                sink.put1(0xe8); // call
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::X86CallPLTRel4,
                                    &ExternalName::LibCall(LibCall::ElfTlsGetAddr),
                                    -4);
                sink.put4(0);
                return;
            }
        }
        // Recipe macho_tls_get_addr
        362 => {
            if let InstructionData::UnaryGlobalValue {
                opcode,
                global_value,
                ..
            } = *inst_data {
                // output %rax
                // clobbers %rdi
                
                // movq gv@tlv(%rip), %rdi
                sink.put1(0x48); // rex
                sink.put1(0x8b); // mov
                modrm_riprel(0b111/*out_reg0*/, sink); // 0x3d
                sink.reloc_external(func.srclocs[inst],
                                    Reloc::MachOX86_64Tlv,
                                    &func.global_values[global_value].symbol_name(),
                                    -4);
                sink.put4(0);
                
                // callq *(%rdi)
                sink.put1(0xff);
                sink.put1(0x17);
                return;
            }
        }
        _ => {},
    }
    if encoding.is_legal() {
        bad_encoding(func, inst);
    }
}
