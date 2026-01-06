#!/bin/bash

VERSION=$( [ -f ../../../VERSION ] && cat ../../../VERSION || echo "devel" )
RELEASE=1

UID_HOST=$(id -u)
GID_HOST=$(id -g)

TOP=$(pwd)
# Resolve %TOP/../..
CRATE_ROOT=$(realpath ${TOP}/../..)

# Debian package creates both udsactor and udsactor-unmanaged packages at once
for debian_version in 12 13; do
    OUTDIR="${TOP}/../bin/debian${debian_version}"
    rm -f ${OUTDIR}/udsactor*.deb  #  Clean previous debs if any
    # Compile first the binary using rustbuilder.py
    echo "=== Building udsactor binaries using rustbuilder.py ==="
    cd ${TOP}
    # Always build
    python3 rustbuilder.py Debian${debian_version}
    
    docker_image="rust-builder-udsactor:Debian${debian_version}"
    # Debian based build inside docker
    
    echo "=== Building for Debian ${debian_version} using ${docker_image} ==="
    
    docker run --rm \
        -u ${UID_HOST}:${GID_HOST} \
        -e IN_DOCKER=1 \
        -e DISTRO=Debian${debian_version} \
        -v ${CRATE_ROOT}:/crate \
        -w /crate/building/linux \
        $docker_image \
        dpkg-buildpackage -b -us -uc
    
    # Move to ../bin/debian${debian_version}
    mkdir -p ${OUTDIR}
    mv ${TOP}/../udsactor*.deb ${OUTDIR}/
done

for distro in Fedora openSUSE; do
    distro_lower=$(echo $distro | tr '[:upper:]' '[:lower:]')
    rpm_root=${TOP}/rpm-${distro_lower}
    install_root=${TOP}/rpm-${distro_lower}-root

    OUTDIR="${TOP}/../bin/${distro_lower}"
    rm -f ${OUTDIR}/udsactor*.rpm  #  Clean previous rpms if any
    # Clean to enforce recompilation
    make -C ${TOP} clean \
        IN_DOCKER=0 \
        DISTRO=$distro \
        DESTDIR=${install_root}
    
    for kind in "" "-unmanaged"; do
        # We need to execute manually the Makefile to copy install files
        
        echo "=== Preparing install files for $distro ==="
        rm -rf "${install_root}"
        mkdir -p ${install_root}
        # Re-run to ensure all files are copied
        make -C ${TOP} install-udsactor${kind} \
            IN_DOCKER=0 \
            DISTRO=$distro \
            DESTDIR=${install_root}
        
        echo "=== Preparing RPM build tree ==="
        rm -rf "${rpm_root}"
        mkdir -p ${rpm_root}/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
        cp ${TOP}/udsactor${kind}.spec ${rpm_root}/SPECS/udsactor${kind}.spec
        
        docker_image="rust-builder-udsactor:$distro"
        
        echo "=== Building for $distro using ${docker_image} ==="
        
        docker run --rm \
            -u ${UID_HOST}:${GID_HOST} \
            -e DISTRO=$distro \
            -v ${CRATE_ROOT}:/crate \
            -w /crate/building/linux \
            $docker_image \
            rpmbuild -bb \
                --define "_topdir /crate/building/linux/rpm-${distro_lower}" \
                --define "version ${VERSION}" \
                --define "release ${RELEASE}" \
                --define "DESTDIR /crate/building/linux/rpm-${distro_lower}-root" \
                /crate/building/linux/rpm-${distro_lower}/SPECS/udsactor${kind}.spec
        
        # Move to ../bin/${distro}
        mkdir -p ${OUTDIR}  # Ensure output dir exists
        cp ${TOP}/rpm-${distro_lower}/RPMS/x86_64/udsactor${kind}*.rpm ${OUTDIR}/
        rpm --addsign ${OUTDIR}/udsactor${kind}*.rpm
    done
done

